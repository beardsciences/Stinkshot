//! File helpers for background jobs: swap a file for a re-written copy only when nobody else
//! has it open or changed it, and run a thread at background priority.

use std::os::windows::fs::{FileTimesExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::time::SystemTime;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FILE_DISPOSITION_INFO, FILE_RENAME_INFO, FileDispositionInfo, FileRenameInfo,
    SetFileInformationByHandle,
};

pub use crate::portable::files::*;

const FILE_READ_DATA: u32 = 0x0001;
const FILE_READ_ATTRIBUTES: u32 = 0x0080;
const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
const DELETE: u32 = 0x0001_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const FILE_SHARE_DELETE: u32 = 0x0000_0004;
const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

/// Replace `original` with `replacement` (a finished file in the same folder) when
/// `original` still has `expect_len` bytes and was last written at `expect_modified`, and no
/// other process has it open. `original` keeps its creation and modification times.
///
/// `original` is held open with no sharing at all while it is checked and swapped, so no
/// other program can open, delete or rename it in between. Through that handle it is parked
/// under `<name>.<pid>.stinkshot-old`, `replacement` is renamed into the free name (never
/// over anything: if a file appeared there meanwhile, that file wins), and the parked
/// original is deleted when the handle closes. If the second rename fails the original is
/// renamed back. On anything but `Replaced` the caller still owns `replacement`.
pub fn replace_if_unchanged(
    original: &Path,
    replacement: &Path,
    expect_len: u64,
    expect_modified: SystemTime,
) -> anyhow::Result<Replace> {
    let held = match std::fs::OpenOptions::new()
        // Read access takes part in sharing checks (attribute-only opens do not), so other
        // handles make this fail, and new opens, deletes and renames fail while it is held.
        .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | DELETE)
        .share_mode(0)
        .open(original)
    {
        Ok(f) => f,
        Err(e)
            if matches!(
                e.raw_os_error(),
                Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION)
            ) =>
        {
            return Ok(Replace::InUse);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Replace::Changed),
        Err(e) => return Err(e.into()),
    };
    let md = held.metadata()?;
    if md.len() != expect_len || md.modified()? != expect_modified {
        return Ok(Replace::Changed);
    }
    let created = md.created().ok();
    let modified = md.modified()?;

    // A virus scanner may still be reading the fresh replacement: share everything.
    let temp = match std::fs::OpenOptions::new()
        .access_mode(DELETE | FILE_WRITE_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(replacement)
    {
        Ok(f) => f,
        Err(e) => {
            log::debug!("replace: open {}: {e}", replacement.display());
            return Ok(Replace::InUse);
        }
    };
    let mut times = std::fs::FileTimes::new().set_modified(modified);
    if let Some(c) = created {
        times = times.set_created(c);
    }
    temp.set_times(times)?;

    let target = std::path::absolute(original)?;
    let Some(file_name) = target.file_name() else {
        return Ok(Replace::Changed);
    };
    let mut parked_name = file_name.to_os_string();
    parked_name.push(format!(".{}{PARKED_SUFFIX}", std::process::id()));
    let parked = target.with_file_name(parked_name);

    if let Err(e) = rename_handle(&held, &parked) {
        log::debug!("replace: park {}: {e}", original.display());
        return Ok(Replace::InUse);
    }
    if let Err(e) = rename_handle(&temp, &target) {
        log::debug!("replace {}: {e}", original.display());
        drop(temp);
        if let Err(back) = rename_handle(&held, &target) {
            // Something new took the name while it was free (another program's save): it is
            // newer than the parked copy, which goes, as its rename-over would have done.
            log::info!(
                "replace: {} was replaced meanwhile; dropping the parked copy ({back})",
                original.display()
            );
            let _ = delete_on_close(&held);
            return Ok(Replace::Changed);
        }
        return Ok(Replace::InUse);
    }
    drop(temp);
    if let Err(e) = delete_on_close(&held) {
        // The swap is done; the parked copy is left for `recover_parked`.
        log::warn!("replace: delete {}: {e}", parked.display());
    }
    drop(held);
    Ok(Replace::Replaced)
}

/// Rename the open `file` to `target`, failing if anything exists there.
fn rename_handle(file: &std::fs::File, target: &Path) -> windows::core::Result<()> {
    let name: Vec<u16> = target.as_os_str().encode_wide_vec();
    // FILE_RENAME_INFO with the name inline; u64 backing keeps the struct aligned.
    let header = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    let bytes = header + name.len() * 2 + 2;
    let mut buf = vec![0u64; bytes.div_ceil(8)];
    // SAFETY: `buf` is large enough and aligned for FILE_RENAME_INFO plus the name (zeroed,
    // so ReplaceIfExists is false and the name is terminated); the handle is live.
    unsafe {
        let info = buf.as_mut_ptr() as *mut FILE_RENAME_INFO;
        (*info).RootDirectory = HANDLE::default();
        (*info).FileNameLength = (name.len() * 2) as u32;
        let dst = (buf.as_mut_ptr() as *mut u8).add(header) as *mut u16;
        std::ptr::copy_nonoverlapping(name.as_ptr(), dst, name.len());
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileRenameInfo,
            buf.as_ptr() as *const core::ffi::c_void,
            (buf.len() * 8) as u32,
        )
    }
}

/// Mark the open `file` (opened with DELETE access) for deletion when its last handle closes.
fn delete_on_close(file: &std::fs::File) -> windows::core::Result<()> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: a valid FILE_DISPOSITION_INFO for a live handle.
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
}

trait EncodeWideVec {
    fn encode_wide_vec(&self) -> Vec<u16>;
}

impl EncodeWideVec for std::ffi::OsStr {
    fn encode_wide_vec(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().collect()
    }
}

/// Run the calling thread in background mode: lowest CPU scheduling, very low I/O and memory
/// priority. For work nobody waits for (re-compressing saved files).
pub fn background_thread() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    };
    // SAFETY: the pseudo handle of the calling thread.
    if let Err(e) = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) } {
        log::debug!("background thread priority: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "stinkshot-files-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn replaces_only_unchanged_unopened_files() {
        let d = dir();
        let orig = d.join("a.png");
        let new = d.join("a.png.tmp");
        std::fs::write(&orig, b"original contents").unwrap();
        let md = std::fs::metadata(&orig).unwrap();
        let (len, modified) = (md.len(), md.modified().unwrap());

        // Wrong expectations: changed.
        std::fs::write(&new, b"new").unwrap();
        assert_eq!(
            replace_if_unchanged(&orig, &new, len + 1, modified).unwrap(),
            Replace::Changed
        );
        // Open elsewhere without delete sharing: in use.
        {
            let _other = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(1) // FILE_SHARE_READ
                .open(&orig)
                .unwrap();
            assert_eq!(
                replace_if_unchanged(&orig, &new, len, modified).unwrap(),
                Replace::InUse
            );
        }
        // Open elsewhere with every sharing flag (a viewer, a scanner): still in use.
        {
            let _other = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(7)
                .open(&orig)
                .unwrap();
            assert_eq!(
                replace_if_unchanged(&orig, &new, len, modified).unwrap(),
                Replace::InUse
            );
        }
        assert_eq!(std::fs::read(&orig).unwrap(), b"original contents");
        // Unchanged and closed: replaced, times kept.
        assert_eq!(
            replace_if_unchanged(&orig, &new, len, modified).unwrap(),
            Replace::Replaced
        );
        assert_eq!(std::fs::read(&orig).unwrap(), b"new");
        assert!(!new.exists());
        assert_eq!(
            std::fs::metadata(&orig).unwrap().modified().unwrap(),
            modified
        );
        assert_eq!(
            std::fs::read_dir(&d).unwrap().count(),
            1,
            "no parked or temp file left"
        );
        // Gone: changed.
        std::fs::write(&new, b"x").unwrap();
        std::fs::remove_file(&orig).unwrap();
        assert_eq!(
            replace_if_unchanged(&orig, &new, len, modified).unwrap(),
            Replace::Changed
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_held_original_cannot_be_deleted_or_renamed() {
        let d = dir().join("held");
        std::fs::create_dir_all(&d).unwrap();
        let orig = d.join("b.png");
        std::fs::write(&orig, b"original").unwrap();
        let held = std::fs::OpenOptions::new()
            .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | DELETE)
            .share_mode(0)
            .open(&orig)
            .unwrap();
        assert!(std::fs::remove_file(&orig).is_err());
        assert!(std::fs::rename(&orig, d.join("c.png")).is_err());
        drop(held);
        let _ = std::fs::remove_dir_all(&d);
    }
}
