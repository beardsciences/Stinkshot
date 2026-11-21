//! Encoder parameters and limits shared by every encoder backend.

// The audio constants and the software limit are only read by the Windows encoder so far.
#![cfg_attr(not(windows), allow(dead_code))]

#[derive(Clone, Copy, Debug)]
pub struct VideoParams {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

pub const AUDIO_RATE: u32 = 48_000;
pub const AUDIO_CHANNELS: u32 = 2;
pub const AUDIO_KBPS: u32 = 192;
/// Highest frame rate recorded without a hardware encoder: the software MFT falls behind
/// above it at full-screen sizes (and 4K at 120 FPS is beyond H.264 level 5.2).
pub const MAX_SOFTWARE_FPS: u32 = 60;
