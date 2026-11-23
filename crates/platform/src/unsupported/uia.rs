//! UI element snapping stand-in: no accessibility tree on this OS yet. The result types and
//! `control_type_name` are shared.

use stinkshot_core::PxPoint;

use crate::Hwnd;
pub use crate::portable::uia::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Element snapper stand-in. It cannot be created on this OS yet, so no value exists.
pub struct ElementSnapper {
    _never: std::convert::Infallible,
}

impl ElementSnapper {
    /// Not supported on this OS yet; always an error.
    pub fn new() -> anyhow::Result<ElementSnapper> {
        Err(crate::unsupported("UI element snapping"))
    }

    pub fn request(&self, _hwnd: Hwnd, _p: PxPoint) {
        match self._never {}
    }

    pub fn latest(&self) -> Option<SnapResult> {
        match self._never {}
    }
}
