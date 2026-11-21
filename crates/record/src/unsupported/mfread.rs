//! Video file reader stand-in: no decoder on this OS yet, so every open is an error. The
//! frame types are shared. `sample_to_bgra` reads a Media Foundation sample and has no
//! counterpart here.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use std::path::Path;

use anyhow::Result;

pub use crate::portable::mfread::*;

/// Cannot be created on this OS yet (`open` always fails).
pub struct Reader {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub duration_hns: i64,
    pub has_audio: bool,
    pub has_video: bool,
    _never: std::convert::Infallible,
}

impl Reader {
    /// Not supported on this OS yet; always an error.
    pub fn open(_path: &Path, _video: bool, _audio: bool) -> Result<Reader> {
        Err(anyhow::anyhow!(
            "reading video files is not supported on this OS yet"
        ))
    }

    pub fn seek(&self, _hns: i64) -> Result<()> {
        match self._never {}
    }

    pub fn read_video(&mut self) -> Result<Option<VideoFrame>> {
        match self._never {}
    }

    pub fn read_audio(&mut self) -> Result<Option<AudioChunk>> {
        match self._never {}
    }
}
