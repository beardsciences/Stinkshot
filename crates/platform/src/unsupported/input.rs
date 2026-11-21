//! Input stand-ins: no pointer queries, synthetic input or global hooks on this OS yet.

use std::sync::mpsc::Sender;
use std::time::Instant;

use stinkshot_core::PxPoint;

use crate::portable::input::fixed_key_label;
pub use crate::portable::input::{InputEvent, MouseButton};

/// (0, 0) on this OS yet.
pub fn cursor_pos() -> PxPoint {
    PxPoint::default()
}

/// Always None on this OS yet.
pub fn try_cursor_pos() -> Option<PxPoint> {
    None
}

/// Always false on this OS yet.
pub fn primary_button_down() -> bool {
    false
}

/// Does nothing on this OS yet.
pub fn scroll_at(_at: PxPoint, _notches: i32) {}

/// The fixed label, or "VKxx" where Windows would ask the keyboard layout.
pub fn key_label(vk: u32, _scan: u32, _extended: bool) -> String {
    fixed_key_label(vk).unwrap_or_else(|| format!("VK{vk:02X}"))
}

/// Global input hook stand-in. It cannot be installed on this OS yet, so no value exists.
pub struct InputHook {
    _never: std::convert::Infallible,
}

impl InputHook {
    /// Not supported on this OS yet; always an error.
    pub fn install(_tx: Sender<(Instant, InputEvent)>) -> anyhow::Result<InputHook> {
        Err(anyhow::anyhow!(
            "the global input hook is not supported on this OS yet"
        ))
    }
}
