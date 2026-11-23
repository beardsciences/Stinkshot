//! Media Foundation stand-in: no media framework on this OS yet, so sessions cannot start.
//! The time and packing helpers are shared. `SendCom`, `make_sample` and `activate_name` wrap
//! COM objects and have no counterpart here.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use anyhow::Result;

pub use crate::portable::mf::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

fn unsupported() -> anyhow::Error {
    stinkshot_platform::unsupported("video encoding and decoding")
}

/// Does nothing on this OS yet (there is no COM to initialise).
pub struct ComGuard(());

impl ComGuard {
    /// Does nothing on this OS yet.
    pub fn new() -> Self {
        ComGuard(())
    }
}

impl Default for ComGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Cannot be created on this OS yet.
pub struct MfSession {
    _never: std::convert::Infallible,
}

/// Cannot be created on this OS yet.
pub struct MfGuard {
    _never: std::convert::Infallible,
}

impl MfGuard {
    /// Not supported on this OS yet; always an error.
    pub fn new() -> Result<Self> {
        Err(unsupported())
    }
}

impl MfSession {
    /// Not supported on this OS yet; always an error.
    pub fn new() -> Result<Self> {
        Err(unsupported())
    }
}

/// Does nothing on this OS yet (the timer resolution is left as it is).
pub struct TimerResolution;

impl TimerResolution {
    /// Does nothing on this OS yet.
    pub fn new() -> Self {
        TimerResolution
    }
}

impl Default for TimerResolution {
    fn default() -> Self {
        Self::new()
    }
}
