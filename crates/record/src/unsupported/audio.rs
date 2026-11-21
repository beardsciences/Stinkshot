//! Audio capture stand-in: no capture API on this OS yet, so there are no devices and a
//! capture cannot start. The source type is shared.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use anyhow::Result;

use crate::dsp::SourceQueue;
pub use crate::portable::audio::*;

/// Always empty on this OS yet.
pub fn list_capture_devices() -> Vec<String> {
    Vec::new()
}

/// Not supported on this OS yet; always an error.
pub fn spawn(
    _source: Source,
    _queue: Arc<Mutex<SourceQueue>>,
    _stop: Arc<AtomicBool>,
    _fatal: Arc<Mutex<Option<String>>>,
) -> Result<JoinHandle<()>> {
    Err(anyhow::anyhow!(
        "audio capture is not supported on this OS yet"
    ))
}
