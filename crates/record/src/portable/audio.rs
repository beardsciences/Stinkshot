//! Audio source selection shared by every capture backend.

// Only the Windows recorder picks a source so far.
#![cfg_attr(not(windows), allow(dead_code))]

#[derive(Clone, Debug)]
pub enum Source {
    /// Default render device, captured in loopback mode.
    Loopback,
    /// Capture device by friendly name (None = default).
    Microphone(Option<String>),
}
