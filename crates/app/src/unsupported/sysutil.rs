//! Stand-in for `sysutil.rs` (Win32 helpers) on other systems. Same API; each function does
//! the closest harmless thing until a real implementation exists.

/// Nothing to attach: a terminal launch already has its console.
pub fn attach_parent_console() {}

/// No native message box yet: the message goes to stderr.
pub fn fatal_box(title: &str, message: &str) {
    eprintln!("{title}: {message}");
}

/// No native question box yet: the answer is No, the safe default.
pub fn confirm_box(_title: &str, _message: &str) -> bool {
    false
}

/// Not known yet: 96 (100% scale).
pub fn system_dpi() -> u32 {
    96
}

/// No trash support yet: an error, so the file is kept.
pub fn recycle(_path: &std::path::Path) -> anyhow::Result<()> {
    Err(stinkshot_platform::unsupported("moving files to the trash"))
}

/// Not found: windows are not looked up by title yet.
pub fn find_own_window(_title: &str) -> Option<stinkshot_platform::Hwnd> {
    None
}

/// Does nothing yet.
pub fn set_window_text(_hwnd: stinkshot_platform::Hwnd, _text: &str) {}

/// A process that can be waited on. Never created off Windows yet.
pub struct ProcessWaiter {
    _never: std::convert::Infallible,
}

impl ProcessWaiter {
    pub fn wait(&self, _timeout: std::time::Duration) -> bool {
        match self._never {}
    }
}

/// No running instance is found yet.
pub fn running_instance() -> Option<ProcessWaiter> {
    None
}

/// Does nothing yet.
pub fn show_window(_hwnd: stinkshot_platform::Hwnd) {}

/// Does nothing yet.
pub fn set_cloaked(_hwnd: stinkshot_platform::Hwnd, _cloaked: bool) {}
