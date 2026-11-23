//! Clipboard stand-in: no clipboard access on this OS yet. Writes fail, reads find nothing.

use stinkshot_core::Rgba;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;
/// `set_allow_history` does nothing on this OS yet.
pub const HISTORY_MARKERS_SUPPORTED: bool = false;

fn unsupported() -> anyhow::Error {
    crate::unsupported("the clipboard")
}

/// Does nothing on this OS yet.
pub fn set_allow_history(_allow: bool) {}

/// Not supported on this OS yet; always an error.
pub fn set_image(_img: &Rgba) -> anyhow::Result<()> {
    Err(unsupported())
}

/// Not supported on this OS yet; always an error.
pub fn set_text(_text: &str) -> anyhow::Result<()> {
    Err(unsupported())
}

/// Not supported on this OS yet; always an error.
pub fn set_file_and_image(_path: &std::path::Path, _img: Option<&Rgba>) -> anyhow::Result<()> {
    Err(unsupported())
}

/// Always None on this OS yet.
pub fn get_text() -> Option<String> {
    None
}

/// Always 0 on this OS yet (the clipboard never seems to change).
pub fn sequence() -> u32 {
    0
}

/// Always None on this OS yet.
pub fn get_image() -> Option<Rgba> {
    None
}

/// Always false on this OS yet.
pub fn has_dibv5() -> bool {
    false
}
