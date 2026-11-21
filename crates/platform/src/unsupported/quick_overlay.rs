//! Quick overlay stand-in: no early frozen-screen window on this OS yet. `QuickInput` is
//! shared.

use stinkshot_core::{PxRect, Rgba};

use crate::Hwnd;
pub use crate::portable::quick_overlay::QuickInput;

/// Quick overlay window stand-in. It cannot be shown on this OS yet, so no value exists.
pub struct QuickOverlay {
    _never: std::convert::Infallible,
}

impl QuickOverlay {
    pub fn input(&self) -> QuickInput {
        match self._never {}
    }
    pub fn hwnd(&self) -> Hwnd {
        match self._never {}
    }
}

/// Not supported on this OS yet; always an error.
pub fn show(
    _frozen: &Rgba,
    _bounds: PxRect,
    _windows: Vec<PxRect>,
    _accent: [u8; 3],
) -> anyhow::Result<QuickOverlay> {
    Err(anyhow::anyhow!(
        "the quick overlay is not supported on this OS yet"
    ))
}
