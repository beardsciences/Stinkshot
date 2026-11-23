//! Screen recording.
//!
//! Pipeline (all on worker threads; `Recorder` methods never block the UI for long):
//! - Video: Windows.Graphics.Capture of the monitor containing `region` (or each monitor
//!   the region spans, composited) -> crop -> overlays (click ripples, key labels, webcam
//!   bubble) composited on the GPU or CPU -> NV12 -> Media Foundation SinkWriter, H.264
//!   with hardware MFT preferred (MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS; NVENC on the
//!   user's RTX 3060 Ti), software fallback. Constant frame rate output: duplicate the last
//!   frame when WGC delivers nothing new (static screen), drop when early.
//! - Audio: WASAPI loopback (default render device) and/or microphone (capture device),
//!   resampled to 48 kHz stereo float, mixed, encoded AAC 192 kbps into the same MP4.
//!   Each source can be muted live (writes silence, keeps sync).
//! - Webcam: Media Foundation source reader on the chosen device, latest frame kept,
//!   drawn as a circle (with white 2px ring) in the configured corner.
//! - Click/key visualizer: `stinkshot_platform::input::InputHook`; clicks become expanding
//!   accent-colored ripples (~400 ms); keys show as a bottom-center chip of current combo
//!   for ~1.2 s after release.
//! - Pause/resume: while paused nothing is written and the timeline offset grows, so the
//!   output is one continuous file without a gap.
//! - Dimensions are rounded down to even numbers (H.264 4:2:0 requirement).
//!
//! # Per-OS boundary
//!
//! The public API (names and signatures) is the same on every OS; only Windows records
//! for now:
//!
//! - Windows implementations stay in place (`src/<module>.rs`) and only build on Windows.
//! - On other OSes each such module is swapped for a stand-in in `src/unsupported/` (see the
//!   `path` attributes below). Stand-ins never panic: calls that return a `Result` return
//!   a `stinkshot_platform::Unsupported` error, device lists are empty and
//!   `primary_monitor_rect` is None. Types that only come from a failed constructor
//!   (`Recorder`, capturer, encoder, reader, webcam) cannot be created there. So
//!   `Recorder::start`, `probe`, `thumbnails`, `frame_at`, `trim`, `to_gif` and `decode_all`
//!   all return an error.
//! - Data types and pure logic that do not depend on the OS live once in `src/portable/`
//!   and are re-exported from the module that always held them, with their unit tests, so
//!   those tests run on every OS.
//! - `convert`, `dsp`, `gifenc`, `media`, `overlay` and `timeline` have no OS calls and
//!   build everywhere as they are (`gifenc` and `media` sit on the reader and encoder).
//!
//! To give a module a real macOS or Linux implementation, add a file for it with the same
//! items (for example `src/macos/encoder.rs`, starting with
//! `pub use crate::portable::encoder::*;`) and narrow the stand-in to the OSes still
//! missing:
//!
//! ```ignore
//! #[cfg_attr(target_os = "macos", path = "macos/encoder.rs")]
//! #[cfg_attr(not(any(windows, target_os = "macos")), path = "unsupported/encoder.rs")]
//! mod encoder;
//! ```
//!
//! Move any logic the new file shares with the Windows one into `portable/` first. An
//! OS-specific dependency goes under `[target.'cfg(..)'.dependencies]` in Cargo.toml.

/// OS-independent types and logic, re-exported by the per-OS modules.
mod portable {
    pub(crate) mod audio;
    pub(crate) mod capture;
    pub(crate) mod encoder;
    pub(crate) mod mf;
    pub(crate) mod mfread;
    pub(crate) mod recorder;
}

// The pure modules below are mostly used by the Windows recorder so far, so parts of them
// have no caller elsewhere.
#[cfg_attr(not(windows), path = "unsupported/audio.rs")]
mod audio;
#[cfg_attr(not(windows), path = "unsupported/capture.rs")]
mod capture;
#[cfg_attr(not(windows), allow(dead_code))]
mod convert;
#[cfg_attr(not(windows), allow(dead_code))]
mod dsp;
#[cfg_attr(not(windows), path = "unsupported/encoder.rs")]
mod encoder;
mod gifenc;
mod media;
#[cfg_attr(not(windows), path = "unsupported/mf.rs")]
mod mf;
#[cfg_attr(not(windows), path = "unsupported/mfread.rs")]
mod mfread;
#[cfg_attr(not(windows), allow(dead_code))]
mod overlay;
#[cfg_attr(not(windows), path = "unsupported/recorder.rs")]
mod recorder;
#[cfg_attr(not(windows), allow(dead_code))]
mod timeline;
#[cfg_attr(not(windows), path = "unsupported/webcam.rs")]
mod webcam;

use std::path::{Path, PathBuf};
use std::time::Duration;

use stinkshot_core::{PxRect, Rgba};

// What works on this OS: each module has `SUPPORTED` (false in its stand-in). A false entry
// means the calls it covers fail with `stinkshot_platform::Unsupported` (lists are empty,
// `primary_monitor_rect` is None). `tests/contract.rs` checks this.
/// `Recorder::start` and `primary_monitor_rect`.
pub const RECORDING_SUPPORTED: bool = recorder::SUPPORTED && capture::SUPPORTED;
/// `probe`, `thumbnails`, `frame_at`, `trim`, `to_gif` and `decode_all`.
pub const VIDEO_FILES_SUPPORTED: bool = mf::SUPPORTED && mfread::SUPPORTED && encoder::SUPPORTED;
/// `list_microphones` and a microphone track.
pub const MICROPHONES_SUPPORTED: bool = audio::SUPPORTED;
/// `list_webcams` and the webcam bubble.
pub const WEBCAMS_SUPPORTED: bool = webcam::SUPPORTED;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WebcamOptions {
    /// None = default device; otherwise a name from `list_webcams`.
    pub device: Option<String>,
    pub diameter: u32,
    pub corner: Corner,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordOptions {
    /// Virtual-desktop physical px.
    pub region: PxRect,
    /// Final .mp4 path (written to a temp sibling and renamed on stop).
    pub output: PathBuf,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub system_audio: bool,
    pub microphone: bool,
    /// None = default device; otherwise a name from `list_microphones`.
    pub mic_device: Option<String>,
    pub webcam: Option<WebcamOptions>,
    pub show_clicks: bool,
    pub show_keys: bool,
    pub include_cursor: bool,
    /// RGBA used for click ripples.
    pub accent: [u8; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    /// Human-readable encoder actually used, e.g. "NVIDIA H.264 Encoder MFT (hardware)".
    pub encoder: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub elapsed: Duration,
    pub paused: bool,
    pub frames: u64,
    pub dropped: u64,
    /// Set if a worker hit a fatal error; the UI should stop and report it.
    pub error: Option<String>,
}

pub use recorder::Recorder;

#[derive(Clone, Debug, PartialEq)]
pub struct MediaInfo {
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub has_audio: bool,
}

pub fn probe(input: &Path) -> anyhow::Result<MediaInfo> {
    media::probe(input)
}

/// `count` evenly spaced frames scaled to `height` px tall (for the trim filmstrip).
pub fn thumbnails(input: &Path, count: usize, height: u32) -> anyhow::Result<Vec<Rgba>> {
    media::thumbnails(input, count, height)
}

/// Single frame at `at` (for the trim preview).
pub fn frame_at(input: &Path, at: Duration, max_height: u32) -> anyhow::Result<Rgba> {
    media::frame_at(input, at, max_height)
}

/// Cut [start, end) into `output` (re-encodes so cuts are frame-accurate; keeps audio).
/// `progress` receives 0.0..=1.0.
pub fn trim(
    input: &Path,
    output: &Path,
    start: Duration,
    end: Duration,
    progress: &dyn Fn(f32),
) -> anyhow::Result<()> {
    media::trim(input, output, start, end, progress)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GifOptions {
    pub fps: u32,
    /// Downscale so width <= this (0 = keep).
    pub max_width: u32,
    pub start: Duration,
    /// None = to the end.
    pub end: Option<Duration>,
    /// imagequant quality 0..=100.
    pub quality: u8,
    pub loop_forever: bool,
}

/// Decode `input` (MP4) and write an optimized GIF: per-frame imagequant palette with
/// dithering, frame-diff transparency to shrink unchanged areas.
pub fn to_gif(
    input: &Path,
    output: &Path,
    opts: &GifOptions,
    progress: &dyn Fn(f32),
) -> anyhow::Result<()> {
    gifenc::to_gif(input, output, opts, progress)
}

/// Friendly device names.
pub fn list_microphones() -> Vec<String> {
    audio::list_capture_devices()
}

pub fn list_webcams() -> Vec<String> {
    webcam::list_names()
}

/// One decoded video frame for `decode_all`.
#[doc(hidden)]
pub struct DecodedFrame<'a> {
    pub pts_hns: i64,
    pub dur_hns: i64,
    pub width: u32,
    pub height: u32,
    /// BGRA, top-down, tightly packed.
    pub bgra: &'a [u8],
}

/// Receives one audio chunk for `decode_all`: presentation time (100 ns) and interleaved
/// 48 kHz stereo PCM16.
#[doc(hidden)]
pub type AudioSink<'a> = &'a mut dyn FnMut(i64, &[i16]);

/// Decode every video frame and all audio of `input` in order (for measurement tools and
/// tests: A/V sync, frame pacing). Either stream may be skipped by passing `None`.
#[doc(hidden)]
pub fn decode_all(
    input: &Path,
    mut on_video: Option<&mut dyn FnMut(DecodedFrame<'_>)>,
    mut on_audio: Option<AudioSink<'_>>,
) -> anyhow::Result<()> {
    if let Some(f) = on_video.as_mut() {
        let mut r = mfread::Reader::open(input, true, false)?;
        while let Some(v) = r.read_video()? {
            f(DecodedFrame {
                pts_hns: v.pts_hns,
                dur_hns: v.dur_hns,
                width: r.width,
                height: r.height,
                bgra: &v.bgra,
            });
        }
    }
    if let Some(f) = on_audio.as_mut() {
        let mut r = mfread::Reader::open(input, false, true)?;
        while let Some(a) = r.read_audio()? {
            let pcm: Vec<i16> = a
                .pcm
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();
            f(a.pts_hns, &pcm);
        }
    }
    Ok(())
}

/// The primary monitor's rectangle in physical virtual-desktop pixels (additive helper; useful
/// for tests and tools that do not want to depend on the platform crate).
pub fn primary_monitor_rect() -> Option<PxRect> {
    capture::primary_monitor_rect()
}
