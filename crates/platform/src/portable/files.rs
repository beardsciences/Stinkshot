//! File helper types and the parked-file recovery, shared by every backend.

use std::path::Path;

/// What `replace_if_unchanged` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replace {
    /// `original` now has the replacement's contents (and its own timestamps).
    Replaced,
    /// Another process has `original` open (or the volume refused the swap); left alone.
    InUse,
    /// `original` is gone or no longer has the expected size and modification time.
    Changed,
}

/// Suffix of the name `original` is parked under during `replace_if_unchanged`, after
/// `.<pid>`. A crash can leave one behind; `recover_parked` puts it back.
pub const PARKED_SUFFIX: &str = ".stinkshot-old";

/// Put back an original that a crash left parked by `replace_if_unchanged` (`parked`
/// ends in `.<pid>.stinkshot-old`): renamed back if its name is still free, otherwise (the
/// swap finished, or something newer took the name) deleted. Parked files of this process
/// are left alone. Returns whether `parked` was such a file.
pub fn recover_parked(parked: &Path) -> bool {
    let Some(name) = parked.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(PARKED_SUFFIX) else {
        return false;
    };
    let Some((orig_name, pid)) = stem.rsplit_once('.') else {
        return false;
    };
    if pid
        .parse::<u32>()
        .ok()
        .is_none_or(|p| p == std::process::id())
        || orig_name.is_empty()
    {
        return false;
    }
    let original = parked.with_file_name(orig_name);
    if original.exists() {
        match std::fs::remove_file(parked) {
            Ok(()) => log::info!("removed leftover {}", parked.display()),
            Err(e) => log::debug!("leftover {}: {e}", parked.display()),
        }
    } else {
        match std::fs::rename(parked, &original) {
            Ok(()) => log::info!("restored {} after an interrupted swap", original.display()),
            Err(e) => log::warn!("restore {}: {e}", original.display()),
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "stinkshot-files-portable-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn parked_originals_are_recovered() {
        let d = dir().join("parked");
        std::fs::create_dir_all(&d).unwrap();
        // Crash after parking: renamed back.
        let parked = d.join("x.png.1.stinkshot-old");
        std::fs::write(&parked, b"x").unwrap();
        assert!(recover_parked(&parked));
        assert_eq!(std::fs::read(d.join("x.png")).unwrap(), b"x");
        assert!(!parked.exists());
        // Crash after the swap: the parked copy goes, the new file stays.
        let parked = d.join("x.png.2.stinkshot-old");
        std::fs::write(&parked, b"old").unwrap();
        assert!(recover_parked(&parked));
        assert_eq!(std::fs::read(d.join("x.png")).unwrap(), b"x");
        assert!(!parked.exists());
        // Not a parked name, or this process's own: untouched.
        let own = d.join(format!("y.png.{}.stinkshot-old", std::process::id()));
        std::fs::write(&own, b"y").unwrap();
        assert!(!recover_parked(&own));
        assert!(!recover_parked(&d.join("x.png")));
        assert!(own.exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
