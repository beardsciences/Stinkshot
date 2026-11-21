//! Monitor description shared by every backend.

use stinkshot_core::PxRect;

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    /// HMONITOR value.
    pub handle: isize,
    /// Full monitor rect, physical px, virtual-desktop coords.
    pub rect: PxRect,
    /// Work area (excludes taskbar).
    pub work: PxRect,
    /// DPI scale, 1.0 = 96 dpi.
    pub scale: f32,
    /// Device name like "\\.\DISPLAY1".
    pub name: String,
    pub primary: bool,
}
