//! Text selection stand-in: no accessibility text API on this OS yet. The result type and
//! the rectangle math are shared.

use crate::Hwnd;
pub use crate::portable::text_selection::*;

/// Not supported on this OS yet; always an error.
pub fn selected_text(
    _fallback: Option<Hwnd>,
    _paragraph: bool,
) -> anyhow::Result<Option<TextSelection>> {
    Err(anyhow::anyhow!(
        "reading the text selection is not supported on this OS yet"
    ))
}
