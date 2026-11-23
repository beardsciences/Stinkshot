//! Screen and window capture stand-ins: no capture API on this OS yet, so every capture is an
//! error and session management does nothing. The result types and image helpers are shared.

use stinkshot_core::{PxPoint, PxRect, Rgba};

use crate::Hwnd;
pub use crate::portable::capture::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

fn unsupported() -> anyhow::Error {
    crate::unsupported("screen capture")
}

/// Not supported on this OS yet; always an error.
pub fn freeze_all(_include_cursor: bool) -> anyhow::Result<Frozen> {
    Err(unsupported())
}

/// Not supported on this OS yet; always an error.
pub fn freeze_rect(_rect: PxRect, _include_cursor: bool) -> anyhow::Result<Rgba> {
    Err(unsupported())
}

/// Not supported on this OS yet; always an error.
pub fn capture_rect(_rect: PxRect, _include_cursor: bool) -> anyhow::Result<Rgba> {
    Err(unsupported())
}

/// Not supported on this OS yet; always an error.
pub fn capture_rect_once(_rect: PxRect, _include_cursor: bool) -> anyhow::Result<Rgba> {
    Err(unsupported())
}

/// Does nothing on this OS yet (there is no capture session to prepare).
pub fn warm_up() {}

/// Does nothing on this OS yet.
#[doc(hidden)]
pub fn warm_up_with(_first_frame_timeout_ms: u32) {}

/// Nothing to release on this OS yet; returns an hour (the "nothing to do" interval).
pub fn release_idle_shadows(max_idle: std::time::Duration) -> std::time::Duration {
    std::time::Duration::from_secs(3600).max(max_idle)
}

/// Does nothing on this OS yet.
pub fn release_cached_capture() {}

/// Stand-in for the Windows duplication lease; there is nothing to hand over on this OS.
#[must_use = "the duplication is returned as soon as the lease is dropped"]
pub struct DuplicationLease(());

/// A lease that holds nothing on this OS yet.
pub fn lend_duplication() -> DuplicationLease {
    DuplicationLease(())
}

/// Not supported on this OS yet; always an error.
pub fn capture_window(_hwnd: Hwnd, _opts: &WindowCaptureOpts) -> anyhow::Result<Rgba> {
    Err(crate::unsupported("window capture"))
}

/// Always None on this OS yet (the pointer reads as hidden).
pub fn cursor_shot() -> Option<CursorShot> {
    None
}

/// Draws nothing on this OS yet (there is no pointer image to read).
pub fn draw_cursor(img: &mut Rgba, origin: PxPoint) {
    if let Some(c) = cursor_shot() {
        draw_cursor_shot(img, origin, &c);
    }
}
