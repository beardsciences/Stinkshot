//! Snipping Tool hook stand-in: there is no Snipping Tool chord to take over on this OS. The
//! chord types and the `ChordFilter` state machine are shared.

use std::sync::mpsc::Sender;

pub use crate::portable::snip::*;

/// Keyboard hook stand-in. It cannot be installed on this OS yet, so no value exists.
pub struct SnipHook {
    _never: std::convert::Infallible,
}

impl SnipHook {
    /// Not supported on this OS yet; always an error.
    pub fn install(_tx: Sender<SnipChord>) -> anyhow::Result<SnipHook> {
        Err(anyhow::anyhow!(
            "the Snipping Tool hook is not supported on this OS yet"
        ))
    }

    pub fn set_pass_through(&self, _on: bool) {
        match self._never {}
    }
}
