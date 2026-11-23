//! MP4 writer stand-in: no encoder on this OS yet, so every create is an error. The
//! parameters and limits are shared.

// Mirrors the Windows module; not every item has a caller off Windows yet.
#![allow(dead_code)]

use std::path::Path;

use anyhow::Result;

pub use crate::portable::encoder::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Cannot be created on this OS yet (`create` always fails).
pub struct Encoder {
    pub encoder_name: String,
    pub fps: u32,
    pub hardware: bool,
    _never: std::convert::Infallible,
}

impl Encoder {
    /// Not supported on this OS yet; always an error.
    pub fn create(_path: &Path, _vp: &VideoParams, _audio: bool) -> Result<Encoder> {
        Err(stinkshot_platform::unsupported("writing video files"))
    }

    pub fn write_video(&self, _nv12: &[u8], _pts_hns: i64, _dur_hns: i64) -> Result<()> {
        match self._never {}
    }

    pub fn write_audio(&self, _pcm: &[u8], _pts_hns: i64, _dur_hns: i64) -> Result<()> {
        match self._never {}
    }

    pub fn finalize(&self) -> Result<()> {
        match self._never {}
    }
}
