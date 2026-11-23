//! OCR stand-in: no OCR engine on this OS yet. The result types and the `OcrEngine` trait are
//! shared, so a real engine only has to implement the trait.

use stinkshot_core::Rgba;

pub use crate::portable::ocr::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Stand-in for the Windows.Media.Ocr engine (the name consumers use). It cannot be created
/// on this OS yet, so no value exists.
pub struct WindowsOcr {
    _never: std::convert::Infallible,
}

impl WindowsOcr {
    /// Not supported on this OS yet; always an error.
    pub fn new() -> anyhow::Result<WindowsOcr> {
        Err(crate::unsupported("OCR"))
    }
}

impl OcrEngine for WindowsOcr {
    fn name(&self) -> &str {
        match self._never {}
    }

    fn recognize(&self, _img: &Rgba) -> anyhow::Result<OcrResult> {
        match self._never {}
    }
}
