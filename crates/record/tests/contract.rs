//! Boundary contract (docs/notes/cross-platform.md): what `stinkshot-record` provides on every
//! OS. Same layout as `crates/platform/tests/contract.rs`:
//!
//! - `shape`: the public signatures, pinned as fn pointers.
//! - `everywhere`: bad input (missing files, empty ranges, a 0x0 region) gives an error and
//!   writes nothing, with any implementation. Device lists are read, nothing is opened.
//! - `stand_ins`: while a `*_SUPPORTED` constant is false, the calls it covers give an
//!   `Unsupported` error, lists are empty and `primary_monitor_rect` is None.

// Signatures are spelled out on purpose: they are the contract.
#![allow(clippy::type_complexity)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use stinkshot_core::{PxRect, Rgba};
use stinkshot_platform::{Unsupported, is_unsupported};
use stinkshot_record::*;

fn assert_unsupported<T>(what: &str, r: anyhow::Result<T>) {
    match r {
        Ok(_) => panic!("{what}: a stand-in returned Ok"),
        Err(e) => assert!(
            e.chain().any(|c| c.downcast_ref::<Unsupported>().is_some()) && is_unsupported(&e),
            "{what}: not an Unsupported error: {e:#}"
        ),
    }
}

/// A fresh, empty, per-test directory.
fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "stinkshot-record-contract-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn options(region: PxRect, output: PathBuf) -> RecordOptions {
    RecordOptions {
        region,
        output,
        fps: 30,
        bitrate_kbps: 8000,
        system_audio: false,
        microphone: false,
        mic_device: None,
        webcam: None,
        show_clicks: false,
        show_keys: false,
        include_cursor: false,
        accent: [0, 0, 0, 255],
    }
}

fn gif_options() -> GifOptions {
    GifOptions {
        fps: 10,
        max_width: 0,
        start: Duration::ZERO,
        end: None,
        quality: 80,
        loop_forever: true,
    }
}

fn no_progress(_: f32) {}

mod shape {
    use super::*;

    #[test]
    fn recorder() {
        let _: fn(RecordOptions) -> anyhow::Result<Recorder> = Recorder::start;
        let _: fn(&Recorder) -> &'static str = Recorder::capture_backend;
        let _: fn(&Recorder) -> u32 = Recorder::fps;
        let _: fn(&Recorder) = Recorder::pause;
        let _: fn(&Recorder) = Recorder::resume;
        let _: fn(&Recorder) -> Status = Recorder::status;
        let _: fn(&Recorder) -> bool = Recorder::can_add_mic;
        let _: fn(&Recorder) -> bool = Recorder::mic_open;
        let _: fn(&mut Recorder, Option<String>) -> anyhow::Result<()> = Recorder::start_mic;
        let _: fn(&Recorder) -> bool = Recorder::low_disk;
        let _: fn(&Recorder, bool) = Recorder::set_mic_muted;
        let _: fn(&Recorder, bool) = Recorder::set_system_muted;
        let _: fn(&Recorder, bool) = Recorder::set_webcam_visible;
        let _: fn(&Recorder, bool, bool) = Recorder::set_overlays;
        let _: fn(Recorder) -> anyhow::Result<RecordingResult> = Recorder::stop;
        let _: fn(Recorder) = Recorder::cancel;
    }

    #[test]
    fn media_and_devices() {
        let _: fn(&Path) -> anyhow::Result<MediaInfo> = probe;
        let _: fn(&Path, usize, u32) -> anyhow::Result<Vec<Rgba>> = thumbnails;
        let _: fn(&Path, Duration, u32) -> anyhow::Result<Rgba> = frame_at;
        let _: fn(&Path, &Path, Duration, Duration, &dyn Fn(f32)) -> anyhow::Result<()> = trim;
        let _: fn(&Path, &Path, &GifOptions, &dyn Fn(f32)) -> anyhow::Result<()> = to_gif;
        let _: fn(
            &Path,
            Option<&mut dyn FnMut(DecodedFrame<'_>)>,
            Option<AudioSink<'_>>,
        ) -> anyhow::Result<()> = decode_all;
        let _: fn() -> Vec<String> = list_microphones;
        let _: fn() -> Vec<String> = list_webcams;
        let _: fn() -> Option<PxRect> = primary_monitor_rect;
        let _: [bool; 4] = [
            RECORDING_SUPPORTED,
            VIDEO_FILES_SUPPORTED,
            MICROPHONES_SUPPORTED,
            WEBCAMS_SUPPORTED,
        ];
    }
}

mod everywhere {
    use super::*;

    #[test]
    fn missing_input_is_an_error_and_writes_nothing() {
        let dir = temp_dir("missing");
        let input = dir.join("missing.mp4");
        assert!(probe(&input).is_err());
        assert!(frame_at(&input, Duration::ZERO, 64).is_err());
        assert!(thumbnails(&input, 3, 32).is_err());
        assert!(thumbnails(&input, 0, 32).unwrap().is_empty());
        let out = dir.join("out.mp4");
        assert!(
            trim(
                &input,
                &out,
                Duration::ZERO,
                Duration::from_secs(1),
                &no_progress
            )
            .is_err()
        );
        assert!(
            trim(
                &input,
                &out,
                Duration::from_secs(1),
                Duration::ZERO,
                &no_progress
            )
            .is_err()
        );
        let gif = dir.join("out.gif");
        assert!(to_gif(&input, &gif, &gif_options(), &no_progress).is_err());
        let mut frames = 0;
        let mut on_video = |_: DecodedFrame<'_>| frames += 1;
        assert!(decode_all(&input, Some(&mut on_video), None).is_err());
        assert_eq!(frames, 0);
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert!(left.is_empty(), "files left behind: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_region_does_not_record() {
        let dir = temp_dir("region");
        let out = dir.join("sub").join("rec.mp4");
        assert!(Recorder::start(options(PxRect::new(0, 0, 0, 0), out)).is_err());
        assert!(!dir.join("sub").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn device_lists_and_monitor() {
        for name in list_microphones().into_iter().chain(list_webcams()) {
            assert!(!name.trim().is_empty());
        }
        if let Some(r) = primary_monitor_rect() {
            assert!(r.w > 0 && r.h > 0, "{r:?}");
        }
    }
}

mod stand_ins {
    use super::*;

    #[test]
    fn recording() {
        if RECORDING_SUPPORTED {
            return;
        }
        let dir = temp_dir("standin");
        let r = Recorder::start(options(PxRect::new(0, 0, 640, 480), dir.join("rec.mp4")));
        assert_unsupported("Recorder::start", r);
        assert!(primary_monitor_rect().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_files() {
        if VIDEO_FILES_SUPPORTED {
            return;
        }
        let input = Path::new("clip.mp4");
        let out = std::env::temp_dir().join("stinkshot-contract-never-written.mp4");
        assert_unsupported("probe", probe(input));
        assert_unsupported("frame_at", frame_at(input, Duration::ZERO, 64));
        assert_unsupported("thumbnails", thumbnails(input, 3, 32));
        let t = trim(
            input,
            &out,
            Duration::ZERO,
            Duration::from_secs(1),
            &no_progress,
        );
        assert_unsupported("trim", t);
        let g = to_gif(input, &out, &gif_options(), &no_progress);
        assert_unsupported("to_gif", g);
        let mut sink = |_: i64, _: &[i16]| {};
        assert_unsupported("decode_all", decode_all(input, None, Some(&mut sink)));
        assert!(!out.exists());
    }

    #[test]
    fn devices() {
        if !MICROPHONES_SUPPORTED {
            assert!(list_microphones().is_empty());
        }
        if !WEBCAMS_SUPPORTED {
            assert!(list_webcams().is_empty());
        }
    }
}
