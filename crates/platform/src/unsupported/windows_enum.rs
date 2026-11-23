//! Window enumeration stand-in: no other windows are listed on this OS yet. `WinInfo` and
//! `window_at` are shared.

use crate::Hwnd;
use crate::occlusion::Layer;
pub use crate::portable::windows_enum::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Always empty on this OS yet.
pub fn layers() -> Vec<Layer> {
    Vec::new()
}

/// Always empty on this OS yet.
pub fn visible_windows() -> Vec<WinInfo> {
    Vec::new()
}

/// Always None on this OS yet.
pub fn foreground() -> Option<WinInfo> {
    None
}

/// Always None on this OS yet.
pub fn info(_hwnd: Hwnd) -> Option<WinInfo> {
    None
}
