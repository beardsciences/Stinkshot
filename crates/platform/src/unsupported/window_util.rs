//! Own-window helper stand-ins: no native window access on this OS yet, so every call is a
//! no-op and lookups find nothing. The "hide from captures" setting is shared.

use stinkshot_core::PxRect;

use crate::Hwnd;
pub use crate::portable::window_util::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Always None on this OS yet.
pub fn find_own_window(_title: &str) -> Option<Hwnd> {
    None
}

/// Does nothing on this OS yet.
pub fn set_rect(_hwnd: Hwnd, _rect: PxRect) {}

/// Always None on this OS yet.
pub fn get_rect(_hwnd: Hwnd) -> Option<PxRect> {
    None
}

/// Does nothing on this OS yet.
pub fn set_topmost(_hwnd: Hwnd, _topmost: bool) {}

/// Does nothing on this OS yet.
pub fn exclude_from_capture(_hwnd: Hwnd, _exclude: bool) {}

/// Does nothing on this OS yet.
pub fn exclude_from_capture_always(_hwnd: Hwnd) {}

/// Does nothing on this OS yet.
pub fn set_hollow_region(_hwnd: Hwnd, _border: i32) {}

/// Does nothing on this OS yet.
pub fn set_no_frame(_hwnd: Hwnd) {}

/// Does nothing on this OS yet.
pub fn apply_mica(_hwnd: Hwnd, _dark: bool) {}

/// Does nothing on this OS yet.
pub fn force_foreground(_hwnd: Hwnd) {}

/// Does nothing on this OS yet.
pub fn set_tool_window(_hwnd: Hwnd, _tool: bool) {}
