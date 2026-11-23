//! Recording capture stand-in: no capture API on this OS yet, so no monitors are listed and
//! a capturer cannot be created. The result types and fallback decisions are shared.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use anyhow::Result;
use stinkshot_core::PxRect;

pub use crate::portable::capture::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Always empty on this OS yet.
pub fn enumerate_monitors() -> Vec<MonitorRect> {
    Vec::new()
}

/// Always None on this OS yet.
pub fn primary_monitor_rect() -> Option<PxRect> {
    None
}

/// Cannot be created on this OS yet (`new` always fails).
pub struct Capturer {
    pub backend_name: &'static str,
    _never: std::convert::Infallible,
}

/// Cannot be created on this OS yet (only a `Capturer` hands one out).
pub struct Mapped<'a> {
    pub data: *const u8,
    pub stride: usize,
    _cap: std::marker::PhantomData<&'a Capturer>,
    _never: std::convert::Infallible,
}

impl Mapped<'_> {
    pub fn bytes(&self, _height: usize) -> &[u8] {
        match self._never {}
    }
}

impl Capturer {
    /// Not supported on this OS yet; always an error.
    pub fn new(_region: PxRect, _include_cursor: bool, _fps: u32) -> Result<Capturer> {
        Err(stinkshot_platform::unsupported(
            "screen capture for recording",
        ))
    }

    pub fn needs_cursor_overlay(&self) -> bool {
        match self._never {}
    }

    pub fn composite_cursor(&self, _bgra: &mut [u8], _w: usize, _h: usize) {
        match self._never {}
    }

    pub fn release_displays(&mut self) {
        match self._never {}
    }

    pub fn has_all_frames(&self) -> bool {
        match self._never {}
    }

    pub fn has_any_frame(&self) -> bool {
        match self._never {}
    }

    pub fn poll(&mut self) -> Result<PollStats> {
        match self._never {}
    }

    pub fn map(&self) -> Result<Mapped<'_>> {
        match self._never {}
    }
}
