//! Recorder stand-in: screen recording is not supported on this OS yet, so `start` always
//! fails and no recorder exists.

use anyhow::Result;

use crate::{RecordOptions, RecordingResult, Status};

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Cannot be created on this OS yet (`start` always fails).
pub struct Recorder {
    _never: std::convert::Infallible,
}

impl Recorder {
    /// Not supported on this OS yet; always an error.
    pub fn start(_opts: RecordOptions) -> Result<Recorder> {
        Err(stinkshot_platform::unsupported("screen recording"))
    }

    pub fn capture_backend(&self) -> &'static str {
        match self._never {}
    }

    pub fn fps(&self) -> u32 {
        match self._never {}
    }

    pub fn pause(&self) {
        match self._never {}
    }

    pub fn resume(&self) {
        match self._never {}
    }

    pub fn status(&self) -> Status {
        match self._never {}
    }

    pub fn can_add_mic(&self) -> bool {
        match self._never {}
    }

    pub fn mic_open(&self) -> bool {
        match self._never {}
    }

    pub fn start_mic(&mut self, _device: Option<String>) -> Result<()> {
        match self._never {}
    }

    pub fn low_disk(&self) -> bool {
        match self._never {}
    }

    pub fn set_mic_muted(&self, _muted: bool) {
        match self._never {}
    }

    pub fn set_system_muted(&self, _muted: bool) {
        match self._never {}
    }

    pub fn set_webcam_visible(&self, _visible: bool) {
        match self._never {}
    }

    pub fn set_overlays(&self, _clicks: bool, _keys: bool) {
        match self._never {}
    }

    pub fn stop(self) -> Result<RecordingResult> {
        match self._never {}
    }

    pub fn cancel(self) {
        match self._never {}
    }
}
