//! System integration stand-ins: autostart, shell, theme, single instance, IPC and session
//! events are not written for this OS yet. The event types are shared.

use std::path::Path;
use std::sync::mpsc::Sender;

pub use crate::portable::system::{EndSessionPhase, SessionEvent};
use crate::unsupported;

// Nothing below is implemented on this OS yet.
/// `set_autostart` and `autostart_enabled` (Start With Windows).
pub const AUTOSTART_SUPPORTED: bool = false;
/// `take_over_printscreen`.
pub const PRINTSCREEN_SUPPORTED: bool = false;
/// `acquire_single_instance`, `send_to_running_instance` and `start_ipc_listener`.
pub const SINGLE_INSTANCE_SUPPORTED: bool = false;
/// `start_session_watcher` and `set_end_session_handler`.
pub const SESSION_EVENTS_SUPPORTED: bool = false;
/// `reveal_in_explorer` and `open_path`.
pub const OPEN_SUPPORTED: bool = false;

/// Never packaged on this OS.
pub fn package_family() -> Option<&'static str> {
    None
}

/// Never packaged on this OS.
pub fn is_packaged() -> bool {
    false
}

/// Never packaged on this OS.
pub fn launched_at_startup() -> bool {
    false
}

/// Not supported on this OS yet; always an error.
pub fn set_autostart(_enabled: bool, _exe: &Path) -> anyhow::Result<()> {
    Err(unsupported("autostart"))
}

/// Always false on this OS yet.
pub fn autostart_enabled() -> bool {
    false
}

/// Not supported on this OS yet; always an error (there is no PrintScreen setting to take).
pub fn take_over_printscreen(_take: bool) -> anyhow::Result<bool> {
    Err(unsupported("taking over PrintScreen"))
}

/// Not supported on this OS yet; always an error.
pub fn reveal_in_explorer(_path: &Path) -> anyhow::Result<()> {
    Err(unsupported("showing a file in the file manager"))
}

/// Not supported on this OS yet; always an error.
pub fn open_path(_path: &Path) -> anyhow::Result<()> {
    Err(unsupported("opening a file"))
}

/// Always false (light) on this OS yet.
pub fn system_dark_mode() -> bool {
    false
}

/// Always None on this OS yet.
pub fn system_accent() -> Option<[u8; 4]> {
    None
}

/// Single-instance guard stand-in. Holds nothing: no other instance is detected.
pub struct InstanceGuard(());

/// Nothing to release yet; kept so callers can end the guard's life on purpose, as on
/// Windows.
impl Drop for InstanceGuard {
    fn drop(&mut self) {}
}

/// Always Some on this OS yet: single instance is not enforced, so every launch runs.
pub fn acquire_single_instance() -> Option<InstanceGuard> {
    Some(InstanceGuard(()))
}

/// Always false on this OS yet (no running instance is found).
pub fn send_to_running_instance(_args: &[String]) -> bool {
    false
}

/// Not supported on this OS yet; always an error.
pub fn start_ipc_listener(_tx: Sender<Vec<String>>) -> anyhow::Result<()> {
    Err(unsupported("the instance IPC listener"))
}

/// Does nothing on this OS yet (`run` is dropped and never called).
pub fn set_end_session_handler(
    _reason: &str,
    _run: impl Fn(EndSessionPhase) -> bool + Send + Sync + 'static,
) {
}

/// Does nothing on this OS yet.
pub fn end_session_work_done() {}

/// Not supported on this OS yet; always an error.
pub fn start_session_watcher(_tx: Sender<SessionEvent>) -> anyhow::Result<()> {
    Err(unsupported("the session watcher"))
}

/// Does nothing on this OS yet.
pub fn play_capture_sound() {}

/// Does nothing on this OS yet.
pub fn trim_working_set() {}
