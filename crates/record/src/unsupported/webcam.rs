//! Webcam stand-in: no camera API on this OS yet, so there are no devices and a camera
//! cannot start.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::overlay::WebcamFrame;

/// Always empty on this OS yet.
pub fn list_names() -> Vec<String> {
    Vec::new()
}

/// Cannot be created on this OS yet (`start` always fails).
pub struct Webcam {
    _never: std::convert::Infallible,
}

impl Webcam {
    /// Not supported on this OS yet; always an error.
    pub fn start(
        _device: Option<&str>,
        _latest: Arc<Mutex<Option<WebcamFrame>>>,
    ) -> Result<Webcam> {
        Err(anyhow::anyhow!(
            "webcam capture is not supported on this OS yet"
        ))
    }
}
