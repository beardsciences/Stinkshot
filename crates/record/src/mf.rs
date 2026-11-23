//! Shared Media Foundation / COM plumbing.

use anyhow::{Context, Result};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::GUID;

pub use crate::portable::mf::*;

/// Implemented on this OS (the stand-ins say false).
pub const SUPPORTED: bool = true;

/// COM initialisation for the current thread (MTA). Tolerates a thread that is already STA.
pub struct ComGuard(bool);

impl ComGuard {
    pub fn new() -> Self {
        // Cached WinRT factories (Graphics.Capture) must not outlive the MTA.
        stinkshot_platform::keep_mta_alive();
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        ComGuard(hr.is_ok())
    }
}

impl Default for ComGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

/// COM for the lifetime of the guard (ref counted) plus process-wide Media Foundation.
pub struct MfSession {
    _mf: MfGuard,
    _com: ComGuard,
}

/// Media Foundation started for the rest of the process. MF is started once and never shut
/// down: each MFStartup/MFShutdown cycle leaks the platform work-queue thread pools (about
/// 50 handles per recording or export). Kept as a token so call sites read the same.
pub struct MfGuard;

static MF_STARTED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();

impl MfGuard {
    pub fn new() -> Result<Self> {
        MF_STARTED
            .get_or_init(|| {
                unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }.map_err(|e| format!("{e}"))
            })
            .clone()
            .map_err(|e| anyhow::anyhow!("MFStartup failed: {e}"))?;
        Ok(MfGuard)
    }
}

impl MfSession {
    pub fn new() -> Result<Self> {
        let com = ComGuard::new();
        let mf = MfGuard::new()?;
        Ok(Self { _mf: mf, _com: com })
    }
}

/// Wrapper to move COM objects (agile / MTA) between threads.
pub struct SendCom<T>(pub T);
unsafe impl<T> Send for SendCom<T> {}
unsafe impl<T> Sync for SendCom<T> {}

/// Build a sample holding a copy of `data`.
pub fn make_sample(data: &[u8], pts_hns: i64, dur_hns: i64) -> Result<IMFSample> {
    unsafe {
        let buffer = MFCreateMemoryBuffer(data.len() as u32).context("MFCreateMemoryBuffer")?;
        let mut ptr = std::ptr::null_mut();
        let mut max = 0u32;
        buffer
            .Lock(&mut ptr, Some(&mut max), None)
            .context("buffer lock")?;
        if ptr.is_null() || (max as usize) < data.len() {
            let _ = buffer.Unlock();
            anyhow::bail!("media buffer too small");
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        buffer.Unlock().context("buffer unlock")?;
        buffer
            .SetCurrentLength(data.len() as u32)
            .context("SetCurrentLength")?;
        let sample = MFCreateSample().context("MFCreateSample")?;
        sample.AddBuffer(&buffer).context("AddBuffer")?;
        sample.SetSampleTime(pts_hns).context("SetSampleTime")?;
        sample
            .SetSampleDuration(dur_hns)
            .context("SetSampleDuration")?;
        Ok(sample)
    }
}

/// Friendly name of an activation object (device sources).
pub fn activate_name(act: &IMFActivate, key: &GUID) -> Option<String> {
    unsafe {
        let mut p = windows::core::PWSTR::null();
        let mut len = 0u32;
        act.GetAllocatedString(key, &mut p, &mut len).ok()?;
        if p.is_null() {
            return None;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p.0, len as usize));
        windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
        Some(s)
    }
}

/// Requests 1 ms system timer resolution while alive.
pub struct TimerResolution;

impl TimerResolution {
    pub fn new() -> Self {
        unsafe {
            windows::Win32::Media::timeBeginPeriod(1);
        }
        TimerResolution
    }
}

impl Default for TimerResolution {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TimerResolution {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::Media::timeEndPeriod(1);
        }
    }
}
