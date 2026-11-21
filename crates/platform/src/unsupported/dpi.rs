//! DPI awareness stand-in: there is no per-process DPI mode to set on this OS yet.

/// Does nothing on this OS yet.
pub fn init() {}

/// Stand-in for the Windows per-thread DPI guard; holds nothing and does nothing.
pub struct ThreadPerMonitorAware(());

impl ThreadPerMonitorAware {
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for ThreadPerMonitorAware {
    fn default() -> Self {
        Self::new()
    }
}
