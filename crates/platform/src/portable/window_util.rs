//! The "hide from captures" setting, shared by every backend.

static HIDE_FROM_CAPTURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Settings > General > Hide Stinkshot From Captures. Windows created afterwards follow it.
pub fn set_hide_from_capture(on: bool) {
    HIDE_FROM_CAPTURE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether windows that ask to be hidden from capture are hidden: the setting, and in debug
/// builds not `STINKSHOT_SHOW_IN_CAPTURE=1` (UI reviews screenshot these windows).
pub fn capture_exclusion_on() -> bool {
    HIDE_FROM_CAPTURE.load(std::sync::atomic::Ordering::Relaxed)
        && !(cfg!(debug_assertions)
            && std::env::var("STINKSHOT_SHOW_IN_CAPTURE").is_ok_and(|v| v == "1"))
}
