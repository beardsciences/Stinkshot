//! Boundary contract (docs/notes/cross-platform.md): what every per-OS `platform` module
//! provides. It runs against whatever implementation this OS compiles, so a new macOS or
//! Linux implementation is held to the same rules:
//!
//! - `shape`: the public signatures, pinned as fn pointers. A mismatch fails to compile.
//! - `everywhere`: calls that are safe with any implementation: invariants of read-only
//!   queries and bad input, which must give an error or nothing, never a panic. No captures,
//!   clipboard access, hooks, registry writes or windows.
//! - `stand_ins`: while a module's `SUPPORTED` is false, every call gives an `Unsupported`
//!   error or the documented harmless default.

// Signatures are spelled out on purpose: they are the contract.
#![allow(clippy::type_complexity)]

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant, SystemTime};

use stinkshot_platform::ocr::OcrEngine;
use stinkshot_platform::*;

/// The stand-in error: typed, and recognizable from its text too.
fn assert_unsupported<T>(what: &str, r: anyhow::Result<T>) {
    match r {
        Ok(_) => panic!("{what}: a stand-in returned Ok"),
        Err(e) => {
            assert!(
                e.downcast_ref::<Unsupported>().is_some(),
                "{what}: not an Unsupported error: {e:#}"
            );
            assert!(is_unsupported(&e), "{what}");
            assert!(is_unsupported_text(&format!("{e:#}")), "{what}");
        }
    }
}

fn rect() -> PxRect {
    PxRect::new(0, 0, 64, 48)
}

fn empty() -> PxRect {
    PxRect::new(10, 10, 0, 0)
}

mod shape {
    use super::*;

    #[test]
    fn capture() {
        use stinkshot_platform::capture::*;
        let _: fn(bool) -> anyhow::Result<Frozen> = freeze_all;
        let _: fn(PxRect, bool) -> anyhow::Result<Rgba> = freeze_rect;
        let _: fn(PxRect, bool) -> anyhow::Result<Rgba> = capture_rect;
        let _: fn(PxRect, bool) -> anyhow::Result<Rgba> = capture_rect_once;
        let _: fn() = warm_up;
        let _: fn(u32) = warm_up_with;
        let _: fn(Duration) -> Duration = release_idle_shadows;
        let _: fn() = release_cached_capture;
        let _: fn() -> DuplicationLease = lend_duplication;
        let _: fn(Hwnd, &WindowCaptureOpts) -> anyhow::Result<Rgba> = capture_window;
        let _: fn() -> Option<CursorShot> = cursor_shot;
        let _: fn(&mut Rgba, PxPoint) = draw_cursor;
        let _: bool = SUPPORTED;
    }

    #[test]
    fn clipboard() {
        use stinkshot_platform::clipboard::*;
        let _: fn(bool) = set_allow_history;
        let _: fn(&Rgba) -> anyhow::Result<()> = set_image;
        let _: fn(&str) -> anyhow::Result<()> = set_text;
        let _: fn(&Path, Option<&Rgba>) -> anyhow::Result<()> = set_file_and_image;
        let _: fn() -> Option<String> = get_text;
        let _: fn() -> u32 = sequence;
        let _: fn() -> Option<Rgba> = get_image;
        let _: fn() -> bool = has_dibv5;
        let _: [bool; 2] = [SUPPORTED, HISTORY_MARKERS_SUPPORTED];
    }

    #[test]
    fn dpi_files_hdr() {
        let _: fn() = dpi::init;
        let _: fn() -> dpi::ThreadPerMonitorAware = dpi::ThreadPerMonitorAware::new;
        let _: fn(&Path, &Path, u64, SystemTime) -> anyhow::Result<files::Replace> =
            files::replace_if_unchanged;
        let _: fn() = files::background_thread;
        let _: fn() -> Vec<hdr::DisplayColor> = hdr::displays;
        let _: fn(&str) -> hdr::DisplayColor = hdr::display_for_device;
        let _: fn(isize) -> hdr::DisplayColor = hdr::display_for_monitor;
        let _: [bool; 2] = [files::SUPPORTED, hdr::SUPPORTED];
    }

    #[test]
    fn input() {
        use stinkshot_platform::input::*;
        let _: fn() -> PxPoint = cursor_pos;
        let _: fn() -> Option<PxPoint> = try_cursor_pos;
        let _: fn() -> bool = primary_button_down;
        let _: fn(PxPoint, i32) = scroll_at;
        let _: fn(u32, u32, bool) -> String = key_label;
        let _: fn(Sender<(Instant, InputEvent)>) -> anyhow::Result<InputHook> = InputHook::install;
        let _: bool = SUPPORTED;
    }

    #[test]
    fn monitors_and_windows() {
        let _: fn() -> Vec<monitors::Monitor> = monitors::list;
        let _: fn() -> PxRect = monitors::virtual_bounds;
        let _: fn(PxPoint) -> Option<monitors::Monitor> = monitors::at_point;
        let _: fn() -> Vec<occlusion::Layer> = windows_enum::layers;
        let _: fn() -> Vec<windows_enum::WinInfo> = windows_enum::visible_windows;
        let _: fn() -> Option<windows_enum::WinInfo> = windows_enum::foreground;
        let _: fn(Hwnd) -> Option<windows_enum::WinInfo> = windows_enum::info;
        let _: [bool; 2] = [monitors::SUPPORTED, windows_enum::SUPPORTED];
    }

    #[test]
    fn ocr_uia_text() {
        let _: fn() -> anyhow::Result<ocr::WindowsOcr> = ocr::WindowsOcr::new;
        // The app shares the engine across threads as a trait object.
        let _: fn(ocr::WindowsOcr) -> Arc<dyn OcrEngine> =
            |o| -> Arc<dyn OcrEngine> { Arc::new(o) };
        let _: fn() -> anyhow::Result<uia::ElementSnapper> = uia::ElementSnapper::new;
        let _: fn(&uia::ElementSnapper, Hwnd, PxPoint) = uia::ElementSnapper::request;
        let _: fn(&uia::ElementSnapper) -> Option<uia::SnapResult> = uia::ElementSnapper::latest;
        let _: fn(Option<Hwnd>, bool) -> anyhow::Result<Option<text_selection::TextSelection>> =
            text_selection::selected_text;
        let _: [bool; 3] = [ocr::SUPPORTED, uia::SUPPORTED, text_selection::SUPPORTED];
    }

    #[test]
    fn overlay_and_snip() {
        use stinkshot_platform::quick_overlay::{QuickInput, QuickOverlay};
        let _: fn(&Rgba, PxRect, Vec<PxRect>, [u8; 3]) -> anyhow::Result<QuickOverlay> =
            quick_overlay::show;
        let _: fn(&QuickOverlay) -> QuickInput = QuickOverlay::input;
        let _: fn(&QuickOverlay) -> Hwnd = QuickOverlay::hwnd;
        let _: fn(Sender<snip::SnipChord>) -> anyhow::Result<snip::SnipHook> =
            snip::SnipHook::install;
        let _: fn(&snip::SnipHook, bool) = snip::SnipHook::set_pass_through;
        let _: [bool; 2] = [quick_overlay::SUPPORTED, snip::SUPPORTED];
    }

    #[test]
    fn system() {
        use stinkshot_platform::system::*;
        let _: fn() -> Option<&'static str> = package_family;
        let _: fn() -> bool = is_packaged;
        let _: fn() -> bool = launched_at_startup;
        let _: fn(bool, &Path) -> anyhow::Result<()> = set_autostart;
        let _: fn() -> bool = autostart_enabled;
        let _: fn(bool) -> anyhow::Result<bool> = take_over_printscreen;
        let _: fn(&Path) -> anyhow::Result<()> = reveal_in_explorer;
        let _: fn(&Path) -> anyhow::Result<()> = open_path;
        let _: fn() -> bool = system_dark_mode;
        let _: fn() -> Option<[u8; 4]> = system_accent;
        let _: fn() -> Option<InstanceGuard> = acquire_single_instance;
        let _: fn(&[String]) -> bool = send_to_running_instance;
        let _: fn(Sender<Vec<String>>) -> anyhow::Result<()> = start_ipc_listener;
        // Generic: checked by compiling a call that never runs.
        let _ = || set_end_session_handler("", |_: EndSessionPhase| true);
        let _: fn() = end_session_work_done;
        let _: fn(Sender<SessionEvent>) -> anyhow::Result<()> = start_session_watcher;
        let _: fn() = play_capture_sound;
        let _: fn() = trim_working_set;
        let _: [bool; 5] = [
            AUTOSTART_SUPPORTED,
            PRINTSCREEN_SUPPORTED,
            SINGLE_INSTANCE_SUPPORTED,
            SESSION_EVENTS_SUPPORTED,
            OPEN_SUPPORTED,
        ];
    }

    #[test]
    fn window_util() {
        use stinkshot_platform::window_util::*;
        let _: fn(&str) -> Option<Hwnd> = find_own_window;
        let _: fn(Hwnd, PxRect) = set_rect;
        let _: fn(Hwnd) -> Option<PxRect> = get_rect;
        let _: fn(Hwnd, bool) = set_topmost;
        let _: fn(Hwnd, bool) = exclude_from_capture;
        let _: fn(Hwnd) = exclude_from_capture_always;
        let _: fn(Hwnd, i32) = set_hollow_region;
        let _: fn(Hwnd, bool) = apply_mica;
        let _: fn(Hwnd) = force_foreground;
        let _: fn(Hwnd, bool) = set_tool_window;
        let _: fn() = keep_mta_alive;
        let _: bool = SUPPORTED;
    }
}

mod everywhere {
    use super::*;

    #[test]
    fn unsupported_error_is_recognizable() {
        let e = unsupported("screen capture");
        assert_eq!(
            e.to_string(),
            "screen capture is not supported on this OS yet"
        );
        assert!(is_unsupported(&e));
        // Still found under context, and after crossing a thread as text.
        let e = e.context("freeze");
        assert!(is_unsupported(&e));
        assert!(is_unsupported_text(&format!("{e:#}")));
        assert!(!is_unsupported(&anyhow::anyhow!(
            "The request is not supported."
        )));
        assert!(!is_unsupported_text("access is denied"));
    }

    #[test]
    fn captures_refuse_bad_input() {
        assert!(capture::freeze_rect(empty(), false).is_err());
        assert!(capture::capture_rect(empty(), false).is_err());
        assert!(capture::capture_rect_once(empty(), false).is_err());
        let opts = capture::WindowCaptureOpts::default();
        assert!(capture::capture_window(0, &opts).is_err());
    }

    #[test]
    fn null_window_is_unknown() {
        assert!(windows_enum::info(0).is_none());
        assert!(window_util::get_rect(0).is_none());
        assert!(window_util::find_own_window("stinkshot contract: no such window").is_none());
    }

    #[test]
    fn monitors_are_consistent() {
        let vb = monitors::virtual_bounds();
        for m in monitors::list() {
            let r = m.rect;
            assert!(r.w > 0 && r.h > 0, "{r:?}");
            assert!(m.scale > 0.0);
            assert!(
                r.x >= vb.x && r.y >= vb.y && r.x + r.w <= vb.x + vb.w && r.y + r.h <= vb.y + vb.h,
                "{r:?} outside {vb:?}"
            );
            let center = PxPoint {
                x: r.x + r.w / 2,
                y: r.y + r.h / 2,
            };
            assert!(monitors::at_point(center).is_some());
        }
        let far = PxPoint {
            x: i32::MIN / 2,
            y: i32::MIN / 2,
        };
        assert!(monitors::at_point(far).is_none());
    }

    #[test]
    fn unknown_display_reads_as_sdr() {
        let d = hdr::display_for_device("\\\\.\\STINKSHOT_NO_SUCH_DISPLAY");
        assert!(!d.advanced_color);
        assert_eq!(d.sdr_white_nits, hdr::SCRGB_UNIT_NITS);
    }

    #[test]
    fn missing_file_is_never_replaced() {
        let dir = std::env::temp_dir().join(format!("stinkshot-contract-{}", std::process::id()));
        let r = files::replace_if_unchanged(
            &dir.join("missing.png"),
            &dir.join("missing.tmp"),
            0,
            SystemTime::UNIX_EPOCH,
        );
        assert!(!matches!(r, Ok(files::Replace::Replaced)));
    }

    #[test]
    fn fixed_key_labels_match() {
        for (vk, label) in [
            (0x0D, "Enter"),
            (0x1B, "Esc"),
            (0x20, "Space"),
            (0x2C, "Print Screen"),
        ] {
            assert_eq!(input::key_label(vk, 0, false), label);
        }
    }
}

mod stand_ins {
    use super::*;

    // Clipboard History markers need a clipboard.
    const _: () = assert!(clipboard::SUPPORTED || !clipboard::HISTORY_MARKERS_SUPPORTED);

    #[test]
    fn capture() {
        if capture::SUPPORTED {
            return;
        }
        assert_unsupported("freeze_all", capture::freeze_all(true));
        assert_unsupported("freeze_rect", capture::freeze_rect(rect(), true));
        assert_unsupported("capture_rect", capture::capture_rect(rect(), true));
        assert_unsupported(
            "capture_rect_once",
            capture::capture_rect_once(rect(), true),
        );
        let opts = capture::WindowCaptureOpts::default();
        assert_unsupported("capture_window", capture::capture_window(1, &opts));
        assert!(capture::cursor_shot().is_none());
        let mut img = Rgba::filled(8, 8, [1, 2, 3, 255]);
        capture::draw_cursor(&mut img, PxPoint::default());
        assert_eq!(img, Rgba::filled(8, 8, [1, 2, 3, 255]));
        capture::warm_up();
        capture::warm_up_with(10);
        capture::release_cached_capture();
        let idle = Duration::from_secs(5);
        assert!(capture::release_idle_shadows(idle) >= idle);
        let _lease = capture::lend_duplication();
    }

    #[test]
    fn clipboard() {
        if clipboard::SUPPORTED {
            return;
        }
        let img = Rgba::filled(2, 2, [0, 0, 0, 255]);
        assert_unsupported("set_image", clipboard::set_image(&img));
        assert_unsupported("set_text", clipboard::set_text("x"));
        assert_unsupported(
            "set_file_and_image",
            clipboard::set_file_and_image(Path::new("x.png"), Some(&img)),
        );
        clipboard::set_allow_history(false);
        assert!(clipboard::get_text().is_none());
        assert!(clipboard::get_image().is_none());
        assert_eq!(clipboard::sequence(), 0);
        assert!(!clipboard::has_dibv5());
    }

    #[test]
    fn files_and_hdr() {
        if !files::SUPPORTED {
            let r = files::replace_if_unchanged(
                Path::new("a"),
                Path::new("b"),
                0,
                SystemTime::UNIX_EPOCH,
            );
            assert_unsupported("replace_if_unchanged", r);
            files::background_thread();
        }
        if !hdr::SUPPORTED {
            assert!(hdr::displays().is_empty());
            let d = hdr::display_for_device("x");
            assert_eq!(d.device, "x");
            assert!(!d.advanced_color);
            assert!(!hdr::display_for_monitor(1).advanced_color);
        }
    }

    #[test]
    fn input() {
        if input::SUPPORTED {
            return;
        }
        assert_eq!(input::cursor_pos(), PxPoint::default());
        assert!(input::try_cursor_pos().is_none());
        assert!(!input::primary_button_down());
        input::scroll_at(PxPoint::default(), 3);
        assert!(!input::key_label(0x41, 0, false).is_empty());
        let (tx, _rx) = channel();
        assert_unsupported("InputHook::install", input::InputHook::install(tx));
    }

    #[test]
    fn monitors_and_windows() {
        if !monitors::SUPPORTED {
            assert!(monitors::list().is_empty());
            assert!(monitors::virtual_bounds().is_empty());
            assert!(monitors::at_point(PxPoint::default()).is_none());
        }
        if !windows_enum::SUPPORTED {
            assert!(windows_enum::layers().is_empty());
            assert!(windows_enum::visible_windows().is_empty());
            assert!(windows_enum::foreground().is_none());
            assert!(windows_enum::info(1).is_none());
        }
        if !window_util::SUPPORTED {
            assert!(window_util::find_own_window("Stinkshot").is_none());
            assert!(window_util::get_rect(1).is_none());
            window_util::set_rect(1, rect());
            window_util::set_topmost(1, true);
            window_util::exclude_from_capture(1, true);
            window_util::exclude_from_capture_always(1);
            window_util::set_hollow_region(1, 2);
            window_util::apply_mica(1, true);
            window_util::force_foreground(1);
            window_util::set_tool_window(1, true);
        }
    }

    #[test]
    fn constructors_fail() {
        if !ocr::SUPPORTED {
            assert_unsupported("WindowsOcr::new", ocr::WindowsOcr::new());
        }
        if !uia::SUPPORTED {
            assert_unsupported("ElementSnapper::new", uia::ElementSnapper::new());
        }
        if !text_selection::SUPPORTED {
            assert_unsupported("selected_text", text_selection::selected_text(None, false));
        }
        if !quick_overlay::SUPPORTED {
            let img = Rgba::filled(4, 4, [0, 0, 0, 255]);
            assert_unsupported(
                "quick_overlay::show",
                quick_overlay::show(&img, rect(), vec![rect()], [1, 2, 3]),
            );
        }
        if !snip::SUPPORTED {
            let (tx, _rx) = channel();
            assert_unsupported("SnipHook::install", snip::SnipHook::install(tx));
        }
    }

    #[test]
    fn system() {
        use stinkshot_platform::system::*;
        if !AUTOSTART_SUPPORTED {
            assert_unsupported("set_autostart", set_autostart(true, Path::new("x")));
            assert!(!autostart_enabled());
            assert!(!launched_at_startup());
        }
        if !PRINTSCREEN_SUPPORTED {
            assert_unsupported("take_over_printscreen", take_over_printscreen(true));
            assert_unsupported("take_over_printscreen", take_over_printscreen(false));
        }
        if !OPEN_SUPPORTED {
            assert_unsupported("reveal_in_explorer", reveal_in_explorer(Path::new("x")));
            assert_unsupported("open_path", open_path(Path::new("x")));
        }
        if !SINGLE_INSTANCE_SUPPORTED {
            // Not enforced: every launch runs, and nothing answers.
            assert!(acquire_single_instance().is_some());
            assert!(!send_to_running_instance(&["quit".to_string()]));
            let (tx, _rx) = channel();
            assert_unsupported("start_ipc_listener", start_ipc_listener(tx));
        }
        if !SESSION_EVENTS_SUPPORTED {
            let (tx, _rx) = channel();
            assert_unsupported("start_session_watcher", start_session_watcher(tx));
            set_end_session_handler("x", |_| panic!("never called"));
            end_session_work_done();
        }
        if !AUTOSTART_SUPPORTED && !OPEN_SUPPORTED {
            // A system with none of these has no packaging or theme query either.
            assert!(!is_packaged());
            assert!(package_family().is_none());
            assert!(!system_dark_mode());
            assert!(system_accent().is_none());
            play_capture_sound();
            trim_working_set();
        }
    }

    #[test]
    fn dpi_and_mta() {
        if capture::SUPPORTED {
            return;
        }
        dpi::init();
        let _aware = dpi::ThreadPerMonitorAware::new();
        keep_mta_alive();
    }
}
