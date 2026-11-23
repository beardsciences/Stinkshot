//! Background work for the shell. Everything slow (encoding, disk, OCR, recording
//! start/stop, export, scrolling capture, history maintenance, update check) runs here and
//! reports back through `WorkerMsg`; every send wakes the UI thread.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, anyhow};
use stinkshot_core::annotate::{self, Doc};
use stinkshot_core::config::Config;
use stinkshot_core::naming::{self, NameParts};
use stinkshot_core::{PxRect, Rgba};
use stinkshot_history::{History, ItemId, ItemKind, NewItem};
use stinkshot_platform::capture::{self, Frozen};
use stinkshot_platform::ocr::OcrEngine;
use stinkshot_platform::windows_enum::WinInfo;
use stinkshot_platform::{clipboard, input, system};
use stinkshot_record::{GifOptions, MediaInfo, RecordOptions, Recorder, RecordingResult};
use stinkshot_stitch::drive::{self, DriveControl, DriveParams};
use stinkshot_stitch::{StitchOptions, Stitcher};

use crate::capture_ops;
use crate::edit_save::{self, EditResult, encode_save};
use crate::errors::{self, report};
use crate::filedialog;
use crate::l10n::tr;
use crate::state::{CaptureOutput, Command, FinishAction, SourceInfo};
use crate::update::{self, Release};

pub type Ocr = Option<Arc<dyn OcrEngine>>;

/// Results posted to the UI thread.
pub enum WorkerMsg {
    /// A command produced by a worker (toasts, follow-up actions).
    Cmd(Command),
    CaptureSaved(Box<CaptureSaved>),
    HistoryChanged,
    /// Window capture finished; the overlay can open in Annotate mode.
    WindowCaptured {
        frozen: Frozen,
        windows: Vec<WinInfo>,
        result: Result<(Rgba, SourceInfo), String>,
    },
    /// The window-capture worker crashed before it could report.
    WindowCaptureFailed(String),
    /// Capture Selected Text: the padded on-screen rect of the selection, None when the
    /// focused app shows no selected text.
    SelectedText(Result<Option<PxRect>, String>),
    /// Image + doc loaded for the editor (re-edit); opens an Editor Window.
    OpenAnnotate {
        base: Rgba,
        doc: Doc,
        mode: &'static str,
        editing: Option<ItemId>,
        source: SourceInfo,
        /// File name for the window title.
        name: Option<String>,
    },
    PinReady {
        image: Rgba,
        title: String,
        item: ItemId,
    },
    /// A history picture loaded for Add to Canvas.
    CanvasPicture(Rgba),
    ScrollProgress {
        frames: usize,
        width: u32,
        height: u32,
    },
    ScrollDone {
        result: Result<Rgba, String>,
        cancelled: bool,
        partial: bool,
        source: SourceInfo,
    },
    RecorderStarted(Result<Recorder, String>),
    RecordStopped(Result<(RecordingResult, MediaInfo), String>),
    ExportProgress(f32),
    Exported(Box<Exported>),
    UpdateChecked(Result<Option<Release>, String>),
    /// Update Now download progress, 0..=100.
    UpdateProgress(u8),
    /// Update Now download finished: the verified installer, or why it failed.
    UpdateDownloaded(Result<std::path::PathBuf, String>),
    /// The update installer ended while this copy was still running (exit code if known).
    InstallerExited(Option<i32>),
    /// The configured History folder that was unavailable can be opened again
    /// (`retry_history_folder`): `folder` is the setting it was opened for, `history` is
    /// None when it still cannot be opened.
    HistoryFolderRetried {
        folder: String,
        history: Option<Arc<History>>,
    },
    /// The answer to `ask_discard`: go on with `then` when `yes`.
    DiscardAnswer {
        then: AfterDiscard,
        yes: bool,
    },
}

/// What waits for the user's answer about unsaved Editor Window edits (`ask_discard`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterDiscard {
    Quit,
    InstallUpdate,
}

pub struct CaptureSaved {
    pub item: Option<ItemId>,
    pub path: Option<PathBuf>,
    pub saved_as: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    pub thumb: Rgba,
    pub copied_image: bool,
    pub copied_text: Option<String>,
    pub mode: &'static str,
    pub source: SourceInfo,
    pub action: FinishAction,
    pub rect: PxRect,
    /// Kept only for `FinishAction::Pin`.
    pub pin_image: Option<Rgba>,
    pub editing: bool,
}

pub struct Exported {
    pub items: Vec<(ItemId, PathBuf, ItemKind)>,
    pub thumb: Option<Rgba>,
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    /// Set when something failed (partial results may still be in `items`).
    pub error: Option<String>,
}

/// Everything a worker needs, cloned per job.
#[derive(Clone)]
pub struct Env {
    pub tx: Sender<WorkerMsg>,
    pub history: Arc<History>,
    pub ocr: Ocr,
    pub cfg: Config,
    pub bg: Sender<BgKick>,
}

impl Env {
    pub fn send(&self, msg: WorkerMsg) {
        let _ = self.tx.send(msg);
        crate::wake::wake();
    }
    pub fn notify(&self, title: &str, body: impl Into<String>, error: bool) {
        self.send(WorkerMsg::Cmd(Command::Notify {
            title: title.to_string(),
            body: body.into(),
            error,
        }));
    }
    pub fn kick_ocr(&self) {
        let _ = self.bg.send(BgKick::Ocr);
    }
}

/// False when the thread could not be started (`f` is dropped unrun).
fn spawn(name: &str, f: impl FnOnce() + Send + 'static) -> bool {
    match std::thread::Builder::new().name(name.to_string()).spawn(f) {
        Ok(_) => true,
        Err(e) => {
            log::error!("cannot spawn worker {name}: {e}");
            false
        }
    }
}

static JOBS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

/// Number of worker jobs (and tracked background threads) still running. The shell waits
/// for this to reach zero on quit so recordings, exports and saves are not cut off.
pub fn jobs_in_flight() -> usize {
    JOBS_IN_FLIGHT.load(Ordering::SeqCst)
}

/// Counts one unit of in-flight work until dropped.
pub struct InFlight(());

impl InFlight {
    pub fn begin() -> InFlight {
        JOBS_IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
        InFlight(())
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        JOBS_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Run `f` on a worker, turning panics into an error toast instead of a dead feature.
fn job(env: &Env, name: &'static str, f: impl FnOnce(&Env) + Send + 'static) {
    job_inner(env, name, f, None);
}

/// Like `job`, but when `f` panics `recover` supplies the terminal message the shell is
/// waiting for, so the state the job owned (scroll HUD, pending capture, recorder
/// Starting/Stopping) is released instead of staying stuck.
fn job_recovering(
    env: &Env,
    name: &'static str,
    f: impl FnOnce(&Env) + Send + 'static,
    recover: impl FnOnce() -> WorkerMsg + Send + 'static,
) {
    job_inner(env, name, f, Some(Box::new(recover)));
}

fn job_inner(
    env: &Env,
    name: &'static str,
    f: impl FnOnce(&Env) + Send + 'static,
    recover: Option<Box<dyn FnOnce() -> WorkerMsg + Send>>,
) {
    let env = env.clone();
    let in_flight = InFlight::begin();
    // Shared with this thread so a job that never starts still releases what it owned.
    let recover = Arc::new(Mutex::new(recover));
    let failed = {
        let env = env.clone();
        let recover = recover.clone();
        move || {
            let recover = recover.lock().ok().and_then(|mut r| r.take());
            if let Some(recover) = recover {
                env.send(recover());
            }
            env.notify(
                tr!("shell.internal.title"),
                tr!("shell.internal.body", hint = errors::log_hint()),
                true,
            );
        }
    };
    let on_panic = failed.clone();
    let started = spawn(name, move || {
        let _in_flight = in_flight;
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&env))).is_err() {
            log::error!("worker {name} panicked");
            on_panic();
        }
    });
    if !started {
        // Out of threads or memory: without the terminal message the shell would wait for
        // this job until restart (pending window capture, scroll HUD, recorder state).
        failed();
    }
}

/// Clipboard failures in words a user can act on. A busy clipboard (another app holding it
/// open, common with clipboard managers and remote desktop) is the usual cause.
pub fn clipboard_error(e: &anyhow::Error) -> String {
    log::warn!("clipboard: {e:#}");
    if errors::classify(e) == errors::Cause::ClipboardBusy {
        tr!("shell.clipboard.busy_saved").to_string()
    } else {
        tr!("shell.clipboard.refused_saved").to_string()
    }
}

/// Like `clipboard_error` for copies that can simply be retried (text, history items); other
/// failures keep their own message.
pub fn clipboard_text_error(e: &anyhow::Error) -> String {
    report("copy", e, tr!("shell.clipboard.refused"))
}

/// Marker error for "no OCR engine" inside item jobs (mapped to `shell.ocr.none.body`).
const NO_OCR_MARK: &str = "no OCR language is installed";

/// Context added to the clipboard step of item copies, so a failure there is not reported as
/// an unreadable file.
const CLIPBOARD_MARK: &str = "put it on the clipboard";

/// Copy failures of history items: the clipboard step failed (busy or refused), or the file is
/// gone / unreadable.
fn item_copy_error(e: &anyhow::Error) -> String {
    let on_clipboard = e.chain().any(|c| c.to_string() == CLIPBOARD_MARK);
    if on_clipboard || errors::classify(e) == errors::Cause::ClipboardBusy {
        clipboard_text_error(e)
    } else {
        report("copy item", e, tr!("shell.item.unreadable"))
    }
}

static LAST_SAVE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

// ------------------------------------------------------------------------------ capture

pub fn finish_capture(env: &Env, out: CaptureOutput) {
    job(env, "capture-save", move |env| {
        let CaptureOutput {
            base,
            doc,
            flattened,
            unframed,
            rect,
            action,
            source,
            mode,
            editing,
        } = out;
        let (width, height) = (flattened.width, flattened.height);

        let target = edit_save::history_target(action, editing);
        let written = edit_save::write_to_history(
            &env.history,
            &env.cfg,
            &EditResult {
                base: &base,
                doc: &doc,
                flat: &flattened,
                unframed: unframed.as_ref(),
                source: &source,
                mode,
            },
            target,
        );
        let mut updated = false;
        // Set when the History folder could not be written: the pixels then go to the
        // default folder as a plain file, or failing that to the clipboard, and the user is
        // told which (below, once the outcome is known).
        let mut history_error = None;
        let (item, mut path) = match written {
            Ok(w) => {
                updated = w.updated;
                crate::recompress::enqueue(&w.path, Some((env.history.clone(), w.id)));
                (Some(w.id), Some(w.path))
            }
            Err(e) => {
                history_error = Some(e);
                (None, None)
            }
        };
        drop(base);
        if history_error.is_some() {
            match save_to_default_folder(&flattened, &env.cfg, &source, env.history.folder()) {
                Some(Ok(p)) => path = Some(p),
                Some(Err(e)) => log::warn!("saving capture to the default folder: {e:#}"),
                None => {}
            }
        }
        // CopyText is about the text; everything else must not lose the picture.
        let rescue_copy =
            history_error.is_some() && path.is_none() && action != FinishAction::CopyText;

        let want_copy =
            edit_save::copies_image(action, env.cfg.general.copy_to_clipboard) || rescue_copy;
        let mut copied_image = false;
        if want_copy {
            match clipboard::set_image(&flattened) {
                Ok(()) => copied_image = true,
                Err(e) => {
                    log::error!("clipboard copy failed: {e:#}");
                    if !rescue_copy {
                        env.notify(tr!("shell.clipboard.failed"), clipboard_error(&e), true);
                    }
                }
            }
        }
        if let Some(e) = &history_error {
            let kept = history_failure_outcome(path.as_deref(), copied_image);
            let body = errors::report_with(
                "saving capture to history",
                e,
                tr!("shell.save.history_failed.body"),
                &kept,
            );
            env.notify(tr!("shell.save.history_failed"), body, true);
        }

        let mut saved_as = None;
        if action == FinishAction::SaveAs {
            let stem = path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Stinkshot".to_string());
            let start_dir = LAST_SAVE_DIR
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| env.history.folder().to_path_buf());
            let fmt = env
                .cfg
                .capture
                .format
                .for_size(flattened.width, flattened.height);
            match filedialog::save_image(&stem, fmt, Some(&start_dir)) {
                Ok(Some(target)) => match encode_save(&flattened, &target, &env.cfg) {
                    Ok(()) => {
                        crate::recompress::enqueue(&target, None);
                        if let (Some(parent), Ok(mut g)) = (target.parent(), LAST_SAVE_DIR.lock()) {
                            *g = Some(parent.to_path_buf());
                        }
                        saved_as = Some(target);
                    }
                    Err(e) => env.notify(
                        tr!("shell.save_as.failed"),
                        report("save as", &e, tr!("shell.save_as.failed.body")),
                        true,
                    ),
                },
                Ok(None) => {}
                Err(e) => env.notify(
                    tr!("shell.save_as.open_failed"),
                    report("save dialog", &e, tr!("shell.save_as.open_failed.body")),
                    true,
                ),
            }
        }

        let mut copied_text = None;
        if action == FinishAction::CopyText {
            match &env.ocr {
                None => {
                    let (title, body) = errors::ocr_missing();
                    env.notify(title, body, true)
                }
                Some(engine) => match engine.recognize(&flattened) {
                    Ok(r) => {
                        let text = r.text.trim().to_string();
                        if text.is_empty() {
                            env.notify(tr!("shell.ocr.no_text"), tr!("shell.ocr.nothing"), false);
                        } else {
                            match clipboard::set_text(&text) {
                                Ok(()) => copied_text = Some(text),
                                Err(e) => env.notify(
                                    tr!("shell.ocr.copy_failed"),
                                    clipboard_text_error(&e),
                                    true,
                                ),
                            }
                        }
                    }
                    Err(e) => env.notify(
                        tr!("shell.ocr.failed"),
                        report("ocr on finish", &e, tr!("shell.ocr.failed.capture")),
                        true,
                    ),
                },
            }
        }

        if env.cfg.general.play_sound {
            system::play_capture_sound();
        }

        let thumb = flattened.thumbnail(360, 220);
        let pin_image = (action == FinishAction::Pin).then_some(flattened);
        env.send(WorkerMsg::CaptureSaved(Box::new(CaptureSaved {
            item,
            path,
            saved_as,
            width,
            height,
            thumb,
            copied_image,
            copied_text,
            mode,
            source,
            action,
            rect,
            pin_image,
            editing: updated,
        })));
        if item.is_some() {
            env.send(WorkerMsg::HistoryChanged);
            if env.cfg.history.ocr_index {
                env.kick_ocr();
            }
        }
    });
}

/// Where a capture goes when the History folder cannot be written: the default folder, as a
/// plain file (History picks it up when that folder is in use again). None when the default
/// folder is the one that just failed.
fn save_to_default_folder(
    flat: &Rgba,
    cfg: &Config,
    source: &SourceInfo,
    failed: &Path,
) -> Option<anyhow::Result<PathBuf>> {
    let dir = Config::default().history_folder();
    let same = |a: &Path, b: &Path| {
        let lower = |p: &Path| {
            p.to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .to_lowercase()
        };
        lower(a) == lower(b)
    };
    if same(&dir, failed) {
        return None;
    }
    Some(save_plain(flat, cfg, source, &dir))
}

/// Write `flat` into `dir` under the configured file name pattern, without indexing it.
fn save_plain(
    flat: &Rgba,
    cfg: &Config,
    source: &SourceInfo,
    dir: &Path,
) -> anyhow::Result<PathBuf> {
    let fmt = cfg.capture.format.for_size(flat.width, flat.height);
    let parts = NameParts {
        now: chrono::Local::now(),
        app: &source.app,
        title: &source.window_title,
        width: flat.width,
        height: flat.height,
    };
    let path = naming::unique_path(dir, &cfg.capture.filename_pattern, &parts, fmt.extension());
    encode_save(flat, &path, cfg)?;
    Ok(path)
}

/// The sentence that says what became of a capture History could not take: saved to
/// `elsewhere`, copied, or not kept.
fn history_failure_outcome(elsewhere: Option<&Path>, copied: bool) -> String {
    match elsewhere.and_then(|p| p.parent()) {
        Some(dir) => tr!(
            "shell.save.history_failed.elsewhere",
            dir = dir.display().to_string()
        ),
        None if copied => tr!("shell.save.history_failed.clipboard").to_string(),
        None => tr!("shell.save.history_failed.lost").to_string(),
    }
}

pub fn capture_window(env: &Env, frozen: Frozen, windows: Vec<WinInfo>, win: WinInfo) {
    job_recovering(
        env,
        "window-capture",
        move |env| {
            let source = capture_ops::source_of(&win);
            let result =
                capture_ops::capture_window_image(&env.cfg, &win, env.cfg.capture.include_cursor)
                    .map(|img| (img, source))
                    .map_err(|e| format!("{e:#}"));
            env.send(WorkerMsg::WindowCaptured {
                frozen,
                windows,
                result,
            });
        },
        || WorkerMsg::WindowCaptureFailed("The capture task crashed.".into()),
    );
}

/// Pixels of padding around a text selection, at 100 % scale.
const SELECTION_PAD: f32 = 6.0;

/// Finds where the focused app's text selection is (UI Automation can be slow, so not on the
/// UI thread). `fallback` is searched when the focused element holds no text.
pub fn find_selected_text(env: &Env, fallback: Option<stinkshot_platform::Hwnd>, paragraph: bool) {
    job_recovering(
        env,
        "selected-text",
        move |env| {
            use stinkshot_platform::{monitors, text_selection};
            let result = text_selection::selected_text(fallback, paragraph)
                .map(|sel| {
                    let sel = sel?;
                    let first = *sel.rects.first()?;
                    let pad = (SELECTION_PAD * capture_ops::scale_at(first)).round() as i32;
                    let mons: Vec<PxRect> = monitors::list().into_iter().map(|m| m.rect).collect();
                    text_selection::capture_rect(&sel.rects, sel.element, pad, &mons)
                })
                .map_err(|e| format!("{e:#}"));
            env.send(WorkerMsg::SelectedText(result));
        },
        || WorkerMsg::SelectedText(Err("The selected-text task crashed.".into())),
    );
}

pub fn ocr_image(env: &Env, img: Rgba) {
    job(env, "ocr-region", move |env| {
        let Some(engine) = &env.ocr else {
            let (title, body) = errors::ocr_missing();
            env.notify(title, body, true);
            return;
        };
        match engine.recognize(&img) {
            Ok(r) => {
                let text = r.text.trim().to_string();
                if text.is_empty() {
                    env.notify(tr!("shell.ocr.no_text"), tr!("shell.ocr.nothing"), false);
                    return;
                }
                match clipboard::set_text(&text) {
                    Ok(()) => env.notify(tr!("shell.saved.text"), preview(&text, 140), false),
                    Err(e) => {
                        env.notify(tr!("shell.ocr.copy_failed"), clipboard_text_error(&e), true)
                    }
                }
            }
            Err(e) => env.notify(
                tr!("shell.ocr.failed"),
                report("ocr region", &e, tr!("shell.ocr.failed.selection")),
                true,
            ),
        }
    });
}

/// First `max` characters, whitespace collapsed, with an ellipsis if cut.
pub fn preview(text: &str, max: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let cut: String = flat.chars().take(max).collect();
        format!("{}...", cut.trim_end())
    }
}

// ------------------------------------------------------------------------- scrolling

pub const SCROLL_RUN: u8 = 0;
pub const SCROLL_STOP: u8 = 1;
pub const SCROLL_CANCEL: u8 = 2;

/// Where a scrolling capture sends its wheel notches.
#[derive(Clone, Copy, Debug)]
pub enum Wheel {
    /// System input at the region's center (`input::scroll_at`): the real path.
    Input,
    /// `WM_MOUSEWHEEL` posted to this window only (soak test window, no system input).
    #[cfg(feature = "soak")]
    Post(stinkshot_platform::Hwnd),
}

impl Wheel {
    fn send(self, center: stinkshot_core::PxPoint, notches: i32) {
        match self {
            Wheel::Input => input::scroll_at(center, notches),
            #[cfg(feature = "soak")]
            Wheel::Post(hwnd) => crate::shell::soak::post_wheel(hwnd, center, notches),
        }
    }
}

pub fn scroll_capture(
    env: &Env,
    rect: PxRect,
    source: SourceInfo,
    control: Arc<AtomicU8>,
    wheel: Wheel,
) {
    job_recovering(
        env,
        "scroll-capture",
        move |env| {
            let cfg = &env.cfg.capture;
            let mut stitcher = Stitcher::new(StitchOptions {
                max_height: cfg.scroll_max_height.max(500),
                ..StitchOptions::default()
            });
            let delay = Duration::from_millis(cfg.scroll_delay_ms.clamp(30, 2000) as u64);
            let center = capture_ops::center(rect);
            let scale = capture_ops::scale_at(rect);
            let mut source = source;
            source.scale = Some(scale);

            // Let the selection overlay and HUD finish disappearing before the first frame.
            std::thread::sleep(Duration::from_millis(250));
            let outcome = drive::drive(
                &mut stitcher,
                &DriveParams {
                    from_top: cfg.scroll_from_top,
                    ..DriveParams::new(rect.h as f32 / scale, delay)
                },
                || capture::capture_rect(rect, false).map_err(|e| format!("{e:#}")),
                |notches| wheel.send(center, notches),
                || match control.load(Ordering::Relaxed) {
                    SCROLL_CANCEL => DriveControl::Cancel,
                    SCROLL_STOP => DriveControl::Stop,
                    _ => DriveControl::Run,
                },
                |stitcher, frame| {
                    env.send(WorkerMsg::ScrollProgress {
                        frames: stitcher.frames(),
                        width: frame.width,
                        height: stitcher.height(),
                    })
                },
            );
            if outcome.cancelled {
                env.send(WorkerMsg::ScrollDone {
                    result: Err("cancelled".into()),
                    cancelled: true,
                    partial: false,
                    source,
                });
                return;
            }
            let (failed, error) = (outcome.failed, outcome.error);
            let frames = stitcher.frames();
            if frames == 0 {
                env.send(WorkerMsg::ScrollDone {
                    result: Err(error.unwrap_or_else(|| "no frames were captured".into())),
                    cancelled: false,
                    partial: false,
                    source,
                });
                return;
            }
            let img = stitcher.finish();
            env.send(WorkerMsg::ScrollDone {
                result: Ok(img),
                cancelled: false,
                partial: failed || error.is_some(),
                source,
            });
        },
        || WorkerMsg::ScrollDone {
            result: Err("The scroll capture task crashed.".into()),
            cancelled: false,
            partial: false,
            source: SourceInfo::default(),
        },
    );
}

// -------------------------------------------------------------------------- recording

pub fn temp_recording_path(history: &History) -> anyhow::Result<PathBuf> {
    let dir = history.folder().join(".stinkshot").join("tmp");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    Ok(dir.join(format!(
        "Recording_{}.mp4",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    )))
}

pub fn start_recorder(env: &Env, opts: RecordOptions) {
    job_recovering(
        env,
        "recorder-start",
        move |env| {
            let r = Recorder::start(opts).map_err(|e| format!("{e:#}"));
            env.send(WorkerMsg::RecorderStarted(r));
        },
        || WorkerMsg::RecorderStarted(Err("The recorder task crashed.".into())),
    );
}

pub fn stop_recorder(env: &Env, rec: Recorder) {
    job_recovering(
        env,
        "recorder-stop",
        move |env| {
            let r = rec.stop().map_err(|e| format!("{e:#}")).and_then(|res| {
                stinkshot_record::probe(&res.path)
                    .map(|info| (res, info))
                    .map_err(|e| format!("{e:#}"))
            });
            env.send(WorkerMsg::RecordStopped(r));
        },
        || WorkerMsg::RecordStopped(Err("The recorder task crashed.".into())),
    );
}

pub fn cancel_recorder(env: &Env, rec: Recorder) {
    job(env, "recorder-cancel", move |_| rec.cancel());
}

fn media_parts(info: &MediaInfo) -> (u32, u32) {
    (info.width, info.height)
}

/// Export the recording at `source` (trimmed to [start, end)) as MP4 and/or GIF into the
/// history folder, index the results and delete the temporary source.
pub fn export_recording(
    env: &Env,
    source: PathBuf,
    start: Duration,
    end: Duration,
    as_gif: bool,
    as_mp4: bool,
) {
    job(env, "recording-export", move |env| {
        let result = export_inner(env, &source, start, end, as_gif, as_mp4);
        let exported = match result {
            Ok(x) => x,
            Err(e) => {
                log::error!("export failed: {e:#}");
                Exported {
                    items: Vec::new(),
                    thumb: None,
                    duration: Duration::ZERO,
                    width: 0,
                    height: 0,
                    bytes: 0,
                    error: Some(format!("{e:#}")),
                }
            }
        };
        env.send(WorkerMsg::Exported(Box::new(exported)));
        env.send(WorkerMsg::HistoryChanged);
    });
}

fn export_inner(
    env: &Env,
    source: &Path,
    start: Duration,
    end: Duration,
    as_gif: bool,
    as_mp4: bool,
) -> anyhow::Result<Exported> {
    let info = stinkshot_record::probe(source).context("read the recording")?;
    let end = if end.is_zero() || end > info.duration {
        info.duration
    } else {
        end
    };
    let start = start.min(end);
    let full =
        start <= Duration::from_millis(40) && end + Duration::from_millis(80) >= info.duration;
    let (w, h) = media_parts(&info);
    let mid = start + (end - start) / 2;
    let thumb = stinkshot_record::frame_at(source, mid, 220).ok();
    let steps = (as_gif as u32 + as_mp4 as u32).max(1) as f32;
    let progress_for = |index: u32| {
        let tx = env.tx.clone();
        move |p: f32| {
            let _ = tx.send(WorkerMsg::ExportProgress(
                ((index as f32 + p.clamp(0.0, 1.0)) / steps).clamp(0.0, 1.0),
            ));
            crate::wake::wake();
        }
    };
    let name_for = |ext: &str| {
        let parts = NameParts {
            now: chrono::Local::now(),
            app: "",
            title: "",
            width: w,
            height: h,
        };
        naming::unique_path(
            env.history.folder(),
            &env.cfg.capture.filename_pattern,
            &parts,
            ext,
        )
    };

    let mut items = Vec::new();
    let mut error: Option<String> = None;
    let mut step = 0u32;
    let mut source_consumed = false;

    if as_gif {
        let dest = name_for("gif");
        let rec = &env.cfg.recording;
        let opts = GifOptions {
            fps: rec.gif_fps.clamp(1, 50),
            max_width: rec.gif_max_width,
            start,
            end: Some(end),
            quality: 90,
            loop_forever: true,
        };
        let p = progress_for(step);
        match stinkshot_record::to_gif(source, &dest, &opts, &p) {
            Ok(()) => items.push((dest, ItemKind::Gif)),
            Err(e) => {
                let _ = std::fs::remove_file(&dest);
                error = Some(format!("GIF export failed: {e:#}"));
            }
        }
        step += 1;
    }
    if as_mp4 {
        let dest = name_for("mp4");
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let outcome = if full {
            let moved = std::fs::rename(source, &dest).or_else(|_| {
                // Different volume: copy, then drop the original.
                // Different volume, or the file is still open elsewhere: copy. A leftover
                // original is swept from the temp folder later.
                std::fs::copy(source, &dest)?;
                remove_recording(source);
                Ok::<(), std::io::Error>(())
            });
            if moved.is_ok() {
                source_consumed = true;
            }
            moved.context("move recording into history")
        } else {
            let p = progress_for(step);
            stinkshot_record::trim(source, &dest, start, end, &p).context("trim recording")
        };
        match outcome {
            Ok(()) => items.push((dest, ItemKind::Video)),
            Err(e) => {
                let _ = std::fs::remove_file(&dest);
                error = Some(format!("MP4 export failed: {e:#}"));
            }
        }
    }

    // Keep the raw recording if nothing could be written.
    if !items.is_empty() && !source_consumed {
        remove_recording(source);
    }

    let mut indexed = Vec::new();
    let mut bytes = 0u64;
    for (path, kind) in items {
        bytes += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let id = env
            .history
            .add(NewItem {
                path: path.clone(),
                kind: Some(kind),
                width: w,
                height: h,
                app: String::new(),
                window_title: String::new(),
                mode: "recording".into(),
            })
            .context("add recording to history")?;
        if let Some(t) = &thumb
            && let Err(e) = env.history.store_thumbnail(id, &t.thumbnail(320, 200))
        {
            log::debug!("store thumbnail: {e:#}");
        }
        indexed.push((id, path, kind));
    }
    if indexed.is_empty() && error.is_none() {
        return Err(anyhow!("nothing was exported"));
    }
    Ok(Exported {
        items: indexed,
        thumb,
        duration: end - start,
        width: w,
        height: h,
        bytes,
        error,
    })
}

/// Save a finished raw recording into history untrimmed, as MP4.
pub fn keep_raw_recording(env: &Env, source: &Path) -> anyhow::Result<()> {
    let x = export_inner(env, source, Duration::ZERO, Duration::ZERO, false, true)?;
    match x.error {
        Some(e) if x.items.is_empty() => Err(anyhow!(e)),
        _ => Ok(()),
    }
}

/// Extension of the marker written beside (and, failing all else, given to) a raw recording
/// that is finished with but could not be deleted. `leftover_recordings` never recovers one.
const DISCARDED_EXT: &str = "discarded";

fn discard_marker(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(DISCARDED_EXT);
    PathBuf::from(name)
}

fn removed(path: &Path) -> bool {
    match std::fs::remove_file(path) {
        Ok(()) => true,
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}

/// Delete a raw recording that was discarded or has been saved. The Trim window's preview
/// readers can hold the file open for a moment after it closes, and a file left behind
/// would come back as a "recovered" recording at the next launch: so it is marked as
/// discarded first, then the delete is retried for a few seconds on a worker.
pub fn remove_recording(path: &Path) {
    if removed(path) {
        return;
    }
    let marker = discard_marker(path);
    if let Err(e) = std::fs::write(&marker, b"") {
        log::warn!("mark {} as discarded: {e}", path.display());
    }
    let path = path.to_path_buf();
    // Counted as in-flight work so a quit waits for the retries.
    let in_flight = InFlight::begin();
    spawn("recording-remove", move || {
        let _in_flight = in_flight;
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(250));
            if removed(&path) {
                let _ = std::fs::remove_file(&marker);
                return;
            }
        }
        // Still held: park it under the marker's name. If even that fails the marker stays
        // and the next launch deletes both.
        match std::fs::rename(&path, &marker) {
            Ok(()) => log::info!("raw recording parked as {}", marker.display()),
            Err(e) => log::warn!("raw recording {} not removed: {e}", path.display()),
        }
    });
}

/// Raw recordings a previous run left in the temp folder: it quit, crashed or lost the GPU
/// before they were saved. Call at startup, before this run can start one of its own.
pub fn leftover_recordings(history: &History) -> Vec<PathBuf> {
    leftover_in(&history.folder().join(".stinkshot").join("tmp"))
}

fn leftover_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let has_ext = |p: &Path, ext: &str| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext));
    let files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    // Finish what `remove_recording` could not: the recording a marker names, then the
    // marker (or parked file) itself.
    for marker in files.iter().filter(|p| has_ext(p, DISCARDED_EXT)) {
        if removed(&marker.with_extension("")) {
            let _ = std::fs::remove_file(marker);
        }
    }
    files
        .into_iter()
        .filter(|p| has_ext(p, "mp4") && p.exists() && !discard_marker(p).exists())
        .collect()
}

/// Save `files` (from `leftover_recordings`) into history. A file that is not a finished
/// recording stays where it is for the weekly temp cleanup.
pub fn recover_recordings(env: &Env, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    job(env, "recording-recover", move |env| {
        let mut saved = 0;
        for f in &files {
            // Another copy of the app sharing this folder may still be writing it.
            if !opens_exclusively(f) {
                log::info!("leftover recording {} is in use, skipped", f.display());
                continue;
            }
            match keep_raw_recording(env, f) {
                Ok(()) => saved += 1,
                Err(e) => log::warn!("leftover recording {} not recovered: {e:#}", f.display()),
            }
        }
        if saved > 0 {
            log::info!("recovered {saved} unsaved recording(s)");
            env.notify(
                tr!("shell.record.recovered"),
                tr!("shell.record.recovered.body"),
                false,
            );
            env.send(WorkerMsg::HistoryChanged);
        }
    });
}

/// Other systems have no exclusive open: a file counts as not in use.
#[cfg(not(windows))]
fn opens_exclusively(_path: &Path) -> bool {
    true
}

#[cfg(windows)]
fn opens_exclusively(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .is_ok()
}

// ------------------------------------------------------------------------ history items

pub fn copy_item_image(env: &Env, id: ItemId) {
    job(env, "copy-image", move |env| {
        let run = || -> anyhow::Result<&'static str> {
            let meta = env
                .history
                .get(id)?
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            match meta.kind {
                ItemKind::Image => {
                    let img = Rgba::load(&meta.path)?;
                    clipboard::set_file_and_image(&meta.path, Some(&img))
                        .context(CLIPBOARD_MARK)?;
                    Ok(tr!("shell.item.image_copied"))
                }
                _ => {
                    clipboard::set_file_and_image(&meta.path, None).context(CLIPBOARD_MARK)?;
                    Ok(tr!("shell.item.file_copied"))
                }
            }
        };
        match run() {
            Ok(title) => env.notify(title, tr!("shell.item.ready"), false),
            Err(e) => env.notify(tr!("shell.pin.copy_failed"), item_copy_error(&e), true),
        }
    });
}

pub fn copy_item_text(env: &Env, id: ItemId) {
    job(env, "copy-text", move |env| {
        let run = || -> anyhow::Result<Option<String>> {
            let meta = env
                .history
                .get(id)?
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            let text = match meta.ocr_text {
                Some(t) if !t.trim().is_empty() => t,
                _ => {
                    if meta.kind != ItemKind::Image {
                        return Ok(None);
                    }
                    let engine = env.ocr.as_ref().ok_or_else(|| anyhow!(NO_OCR_MARK))?;
                    let img = Rgba::load(&meta.path)?;
                    let text = engine.recognize(&img)?.text.trim().to_string();
                    env.history.set_ocr_text(id, &text)?;
                    text
                }
            };
            if text.trim().is_empty() {
                return Ok(None);
            }
            clipboard::set_text(text.trim()).context(CLIPBOARD_MARK)?;
            Ok(Some(text))
        };
        match run() {
            Ok(Some(t)) => env.notify(tr!("shell.saved.text"), preview(&t, 140), false),
            Ok(None) => env.notify(tr!("shell.ocr.no_text"), tr!("shell.ocr.item_none"), false),
            Err(e) if format!("{e:#}").contains(NO_OCR_MARK) => {
                let (title, body) = errors::ocr_missing();
                env.notify(title, body, true)
            }
            Err(e) => env.notify(tr!("shell.ocr.copy_failed"), item_copy_error(&e), true),
        }
    });
}

pub fn pin_item(env: &Env, id: ItemId) {
    job(env, "pin-item", move |env| {
        let run = || -> anyhow::Result<(Rgba, String)> {
            let meta = env
                .history
                .get(id)?
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            if meta.kind != ItemKind::Image {
                return Err(anyhow!("only images can be pinned"));
            }
            let title = meta
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok((Rgba::load(&meta.path)?, title))
        };
        match run() {
            Ok((image, title)) => env.send(WorkerMsg::PinReady {
                image,
                title,
                item: id,
            }),
            Err(e) => env.notify(
                tr!("shell.item.pin_failed"),
                report("pin item", &e, tr!("shell.item.pin_failed.body")),
                true,
            ),
        }
    });
}

/// Load an image item's picture (with its annotations) for Add to Canvas.
pub fn add_to_canvas(env: &Env, id: ItemId) {
    job(env, "add-to-canvas", move |env| {
        let run = || -> anyhow::Result<Rgba> {
            let meta = env
                .history
                .get(id)?
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            if meta.kind != ItemKind::Image {
                return Err(anyhow!("only images can go on a canvas"));
            }
            Rgba::load(&meta.path)
        };
        match run() {
            Ok(img) => env.send(WorkerMsg::CanvasPicture(img)),
            Err(e) => env.notify(
                tr!("shell.item.canvas_failed"),
                report("add to canvas", &e, tr!("shell.item.in_use")),
                true,
            ),
        }
    });
}

pub fn edit_item(env: &Env, id: ItemId) {
    job(env, "edit-item", move |env| {
        let mut name = None;
        let mut run = || -> anyhow::Result<(Rgba, Doc, SourceInfo)> {
            let meta = env
                .history
                .get(id)?
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            if meta.kind != ItemKind::Image {
                return Err(anyhow!("only images can be edited"));
            }
            let source = SourceInfo {
                app: meta.app.clone(),
                window_title: meta.window_title.clone(),
                // History does not record the capture's scale: no logical px readout.
                scale: None,
            };
            name = meta
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            if let Some((base, doc)) = annotate::load_editable(&meta.path)? {
                return Ok((base, doc, source));
            }
            let img = Rgba::load(&meta.path)?;
            let doc = Doc::new(img.width, img.height);
            Ok((img, doc, source))
        };
        match run() {
            Ok((base, doc, source)) => {
                env.send(WorkerMsg::OpenAnnotate {
                    base,
                    doc,
                    mode: "region",
                    editing: Some(id),
                    source,
                    name,
                });
            }
            Err(e) => env.notify(
                tr!("shell.item.edit_failed"),
                report("edit item", &e, tr!("shell.item.edit_failed.body")),
                true,
            ),
        }
    });
}

pub fn delete_item(env: &Env, id: ItemId) {
    job(env, "delete-item", move |env| {
        // Never between a background re-compression's check and its swap.
        let _g = crate::recompress::write_lock();
        // Recycle Bin first so a mistaken delete can be undone from Explorer; the index row
        // and the re-edit sidecars go with `History::delete`.
        if let Ok(Some(meta)) = env.history.get(id)
            && let Err(e) = crate::sysutil::recycle(&meta.path)
        {
            let why = report("recycle", &e, tr!("shell.item.in_use"));
            env.notify(
                tr!("shell.item.delete_failed"),
                tr!("shell.item.recycle_failed", why = why),
                true,
            );
            env.send(WorkerMsg::HistoryChanged);
            return;
        }
        if let Err(e) = env.history.delete(id) {
            let body = report("history delete", &e, tr!("shell.item.forget_failed.body"));
            env.notify(tr!("shell.item.delete_failed"), body, true);
        }
        env.send(WorkerMsg::HistoryChanged);
    });
}

/// Run a shell action on one history item off the UI thread (Explorer can take seconds).
fn with_item_job(
    env: &Env,
    name: &'static str,
    id: ItemId,
    f: impl FnOnce(&Path) -> anyhow::Result<()> + Send + 'static,
) {
    job(env, name, move |env| match env.history.get(id) {
        Ok(Some(meta)) => {
            if let Err(e) = f(&meta.path) {
                let body = report(name, &e, tr!("shell.item.open_failed.body"));
                env.notify(tr!("shell.item.open_failed"), body, true);
            }
        }
        Ok(None) => env.notify(
            tr!("shell.item.not_found"),
            tr!("shell.item.not_found.body"),
            true,
        ),
        Err(e) => {
            let body = report(name, &e, tr!("shell.history.read_failed.body"));
            env.notify(tr!("shell.history.read_failed"), body, true)
        }
    });
}

pub fn reveal_item(env: &Env, id: ItemId) {
    with_item_job(env, "reveal-item", id, system::reveal_in_explorer);
}

pub fn open_item(env: &Env, id: ItemId) {
    with_item_job(env, "open-item", id, system::open_path);
}

pub fn open_url(env: &Env, url: String) {
    job(env, "open-url", move |env| {
        if let Err(e) = system::open_path(Path::new(&url)) {
            let body = report("open url", &e, tr!("shell.url.failed.body"));
            env.notify(tr!("shell.url.failed"), body, true);
        }
    });
}

// ---------------------------------------------------------------- background maintenance

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgKick {
    /// Re-run the OCR indexer.
    Ocr,
    /// Rescan + retention + OCR.
    Full,
}

/// Settings the background thread reads on every pass.
#[derive(Clone)]
pub struct BgConfig {
    pub history: Arc<History>,
    pub ocr: Ocr,
    pub ocr_index: bool,
    pub keep_days: u32,
    pub max_bytes: u64,
}

impl BgConfig {
    pub fn from(cfg: &Config, history: Arc<History>, ocr: Ocr) -> Self {
        Self {
            history,
            ocr,
            ocr_index: cfg.history.ocr_index,
            keep_days: cfg.history.keep_days,
            max_bytes: (cfg.history.max_gb.max(0.0) as f64 * 1024.0 * 1024.0 * 1024.0) as u64,
        }
    }
}

/// History maintenance thread: rescan + retention on start and hourly, OCR indexing on
/// demand. Returns the kick channel; dropping it stops the thread.
pub fn start_background(cfg: Arc<Mutex<BgConfig>>, tx: Sender<WorkerMsg>) -> Sender<BgKick> {
    let (kick_tx, kick_rx): (Sender<BgKick>, Receiver<BgKick>) = channel();
    spawn("history-maintenance", move || {
        let mut kind = BgKick::Full;
        loop {
            let snapshot = cfg.lock().map(|g| g.clone()).ok();
            if let Some(c) = snapshot {
                // One unreadable file must not end retention and indexing until restart.
                let pass = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if kind == BgKick::Full {
                        maintain(&c, &tx);
                    }
                    index_ocr(&c, &tx);
                }));
                if pass.is_err() {
                    log::error!("history maintenance pass panicked; trying again next pass");
                }
            }
            kind = match kick_rx.recv_timeout(Duration::from_secs(3600)) {
                Ok(BgKick::Full) => BgKick::Full,
                Ok(BgKick::Ocr) => {
                    // Coalesce a burst of kicks.
                    let mut k = BgKick::Ocr;
                    while let Ok(next) = kick_rx.try_recv() {
                        if next == BgKick::Full {
                            k = BgKick::Full;
                        }
                    }
                    k
                }
                Err(RecvTimeoutError::Timeout) => BgKick::Full,
                Err(RecvTimeoutError::Disconnected) => return,
            };
        }
    });
    kick_tx
}

fn post(tx: &Sender<WorkerMsg>, msg: WorkerMsg) {
    let _ = tx.send(msg);
    crate::wake::wake();
}

fn maintain(c: &BgConfig, tx: &Sender<WorkerMsg>) {
    let mut changed = false;
    match c.history.rescan() {
        Ok((added, removed)) => {
            if added + removed > 0 {
                log::info!("history rescan: {added} added, {removed} removed");
                changed = true;
            }
        }
        Err(e) => log::warn!("history rescan failed: {e:#}"),
    }
    if c.keep_days > 0 || c.max_bytes > 0 {
        let retained = {
            let _g = crate::recompress::write_lock();
            c.history.apply_retention(c.keep_days, c.max_bytes)
        };
        match retained {
            Ok(n) if n > 0 => {
                log::info!("retention removed {n} items");
                changed = true;
            }
            Ok(_) => {}
            Err(e) => log::warn!("retention failed: {e:#}"),
        }
    }
    clean_temp_recordings(&c.history);
    if changed {
        post(tx, WorkerMsg::HistoryChanged);
    }
}

/// Raw recordings that could not be recovered at startup (`recover_recordings`) are kept a
/// week, then removed.
fn clean_temp_recordings(history: &History) {
    let dir = history.folder().join(".stinkshot").join("tmp");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now() - Duration::from_secs(7 * 24 * 3600);
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t < cutoff)
            .unwrap_or(false);
        if old && let Err(err) = std::fs::remove_file(e.path()) {
            log::debug!("temp cleanup {}: {err}", e.path().display());
        }
    }
}

fn index_ocr(c: &BgConfig, tx: &Sender<WorkerMsg>) {
    let Some(engine) = &c.ocr else { return };
    if !c.ocr_index {
        return;
    }
    let mut done_since_post = 0;
    // Items the recognizer failed on in this pass: skipped so they cannot hold up the rest.
    let mut skipped = std::collections::HashSet::new();
    loop {
        let batch = match c.history.pending_ocr(25 + skipped.len()) {
            Ok(mut b) => {
                b.retain(|m| !skipped.contains(&m.id));
                b
            }
            Err(e) => {
                log::warn!("pending_ocr failed: {e:#}");
                return;
            }
        };
        if batch.is_empty() {
            break;
        }
        let mut stored = 0;
        for item in batch {
            // An image that cannot be read is indexed as empty. A recognizer failure may be
            // temporary, so that item stays pending for a later pass.
            let text = match Rgba::load(&item.path) {
                Ok(img) => match engine.recognize(&img) {
                    Ok(r) => r.text,
                    Err(e) => {
                        log::debug!("ocr index {}: {e:#}", item.path.display());
                        skipped.insert(item.id);
                        continue;
                    }
                },
                Err(e) => {
                    log::debug!("ocr index {}: {e:#}", item.path.display());
                    String::new()
                }
            };
            if c.history.set_ocr_text(item.id, text.trim()).is_ok() {
                stored += 1;
                done_since_post += 1;
            }
            if done_since_post >= 10 {
                done_since_post = 0;
                post(tx, WorkerMsg::HistoryChanged);
            }
            // Stay out of the way of the UI and capture workers.
            std::thread::sleep(Duration::from_millis(25));
        }
        if stored == 0 {
            log::debug!("ocr indexer made no progress; stopping this pass");
            break;
        }
    }
    if done_since_post > 0 {
        post(tx, WorkerMsg::HistoryChanged);
    }
}

/// Wait for the update installer. A successful install quits this copy first, so a message
/// from here means the install was cancelled or failed.
pub fn watch_installer(tx: Sender<WorkerMsg>, mut child: std::process::Child) {
    spawn("update-installer", move || {
        let code = child.wait().ok().and_then(|s| s.code());
        post(&tx, WorkerMsg::InstallerExited(code));
    });
}

pub fn check_updates(tx: Sender<WorkerMsg>) {
    let tx2 = tx.clone();
    let started = spawn("update-check", move || {
        let r = update::check().map_err(|e| format!("{e:#}"));
        post(&tx, WorkerMsg::UpdateChecked(r));
    });
    if !started {
        // The shell waits for this answer before it checks again.
        post(&tx2, WorkerMsg::UpdateChecked(Err(NOT_STARTED.into())));
    }
}

/// First start of another version: remove the setup.exe the update left in `%TEMP%`. The
/// installer that started this copy may still be running from it, so try for a while.
pub fn remove_update_download() {
    spawn("update-cleanup", || {
        for _ in 0..6 {
            if update::remove_stale_download() {
                return;
            }
            std::thread::sleep(Duration::from_secs(20));
        }
    });
}

/// Error text when a worker thread could not be started.
const NOT_STARTED: &str = "the task could not be started";

/// Try to open the configured History folder `dir` that was unavailable (`folder` is the
/// setting's text), off the UI thread: an unreachable share can take many seconds to fail.
pub fn retry_history_folder(tx: Sender<WorkerMsg>, folder: String, dir: PathBuf) {
    let (tx2, folder2) = (tx.clone(), folder.clone());
    let started = spawn("history-folder-retry", move || {
        let history = History::open(&dir).ok().map(Arc::new);
        post(&tx, WorkerMsg::HistoryFolderRetried { folder, history });
    });
    if !started {
        post(
            &tx2,
            WorkerMsg::HistoryFolderRetried {
                folder: folder2,
                history: None,
            },
        );
    }
}

/// Ask whether unsaved Editor Window edits may be discarded (`message`), on its own thread
/// so the UI, a recording and its HUD keep running behind the question.
pub fn ask_discard(tx: Sender<WorkerMsg>, then: AfterDiscard, message: String) {
    let tx2 = tx.clone();
    let started = spawn("discard-question", move || {
        let yes = crate::sysutil::confirm_box("Stinkshot", &message);
        post(&tx, WorkerMsg::DiscardAnswer { then, yes });
    });
    if !started {
        // Unanswered counts as No: nothing is discarded.
        post(&tx2, WorkerMsg::DiscardAnswer { then, yes: false });
    }
}

/// Update Now: download and verify `inst` (see `update::download_installer`).
pub fn download_update(tx: Sender<WorkerMsg>, inst: update::Installer) {
    let tx2 = tx.clone();
    let started = spawn("update-download", move || {
        let mut last = 0u8;
        let r = update::download_installer(&inst, |done, total| {
            let pct = (done.saturating_mul(100) / total.max(1)).min(100) as u8;
            if pct != last {
                last = pct;
                post(&tx, WorkerMsg::UpdateProgress(pct));
            }
        })
        .map_err(|e| format!("{e:#}"));
        post(&tx, WorkerMsg::UpdateDownloaded(r));
    });
    if !started {
        post(&tx2, WorkerMsg::UpdateDownloaded(Err(NOT_STARTED.into())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn a_discarded_recording_held_open_does_not_come_back() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = std::env::temp_dir().join(format!("stinkshot-discard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (kept, held, free) = (dir.join("a.mp4"), dir.join("b.mp4"), dir.join("c.mp4"));
        for p in [&kept, &held, &free] {
            std::fs::write(p, b"x").unwrap();
        }
        // Not in use: gone at once, no marker.
        remove_recording(&free);
        assert!(!free.exists() && !discard_marker(&free).exists());
        // Held without delete sharing, as the preview readers hold it.
        let handle = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&held)
            .unwrap();
        remove_recording(&held);
        assert!(held.exists() && discard_marker(&held).exists());
        assert_eq!(leftover_in(&dir), vec![kept.clone()]);
        // Released: the retry (or the next launch's listing) removes file and marker.
        drop(handle);
        let end = std::time::Instant::now() + Duration::from_secs(5);
        while (held.exists() || discard_marker(&held).exists()) && std::time::Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!held.exists() && !discard_marker(&held).exists());
        // A marker left by a run that was killed mid-retry is acted on at the next launch.
        std::fs::write(&held, b"x").unwrap();
        std::fs::write(discard_marker(&held), b"").unwrap();
        assert_eq!(leftover_in(&dir), vec![kept.clone()]);
        assert!(!held.exists() && !discard_marker(&held).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_capture_history_cannot_take_is_saved_as_a_plain_file() {
        let dir = std::env::temp_dir().join(format!("stinkshot-test-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = Rgba::filled(8, 6, [10, 20, 30, 255]);
        // The folder does not exist yet: it is created.
        let path = save_plain(&img, &Config::default(), &SourceInfo::default(), &dir).unwrap();
        assert_eq!(path.parent(), Some(dir.as_path()));
        let back = Rgba::load(&path).unwrap();
        assert_eq!((back.width, back.height), (8, 6));
        // A second one never overwrites the first.
        let second = save_plain(&img, &Config::default(), &SourceInfo::default(), &dir).unwrap();
        assert_ne!(second, path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_folder_is_not_a_fallback_for_itself() {
        let img = Rgba::filled(2, 2, [0, 0, 0, 255]);
        let default = Config::default().history_folder();
        let cfg = Config::default();
        let src = SourceInfo::default();
        assert!(save_to_default_folder(&img, &cfg, &src, &default).is_none());
        // Case and a trailing separator do not make it another folder.
        let shouted = PathBuf::from(format!("{}\\", default.display()).to_uppercase());
        assert!(save_to_default_folder(&img, &cfg, &src, &shouted).is_none());
    }

    #[test]
    fn history_failure_says_what_became_of_the_capture() {
        let saved = history_failure_outcome(Some(Path::new("C:\\Shots\\a.png")), false);
        assert!(saved.contains("C:\\Shots"), "{saved}");
        assert!(!saved.contains("a.png"));
        let copied = history_failure_outcome(None, true);
        assert!(copied.to_lowercase().contains("clipboard"), "{copied}");
        let lost = history_failure_outcome(None, false);
        assert_ne!(lost, copied);
        assert!(!lost.to_lowercase().contains("saved to"));
    }

    #[test]
    fn a_failed_job_sends_its_terminal_message_once() {
        // A thread that cannot be started takes the same path as a panicking job; only the
        // panic can be provoked here.
        let (tx, rx) = channel();
        let (bg, _bg_rx) = channel();
        let dir = std::env::temp_dir().join(format!("stinkshot-test-job-{}", std::process::id()));
        let env = Env {
            tx,
            history: Arc::new(History::open(&dir).unwrap()),
            ocr: None,
            cfg: Config::default(),
            bg,
        };
        job_recovering(
            &env,
            "test-job",
            |_| panic!("boom"),
            || WorkerMsg::WindowCaptureFailed("recovered".into()),
        );
        let first = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(matches!(first, WorkerMsg::WindowCaptureFailed(t) if t == "recovered"));
        let second = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(matches!(
            second,
            WorkerMsg::Cmd(Command::Notify { error: true, .. })
        ));
        drop(env);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_framed_capture_keeps_its_unframed_picture_for_re_editing() {
        let dir = std::env::temp_dir().join(format!("stinkshot-sidecar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = Rgba::filled(40, 30, [10, 20, 30, 255]);
        let framed = stinkshot_core::beautify::apply(
            &base,
            &stinkshot_core::beautify::Beautify {
                padding: 16,
                ..Default::default()
            },
        );
        assert_ne!((framed.width, framed.height), (40, 30));
        for keep_editable in [true, false] {
            let mut cfg = Config::default();
            cfg.history.keep_editable = keep_editable;
            let path = dir.join(format!("shot-{keep_editable}.png"));
            std::fs::write(
                &path,
                framed
                    .encode(stinkshot_core::image::ImageFormat::Png, 90)
                    .unwrap(),
            )
            .unwrap();
            // No annotations, Beautify applied: the re-edit base is the original size.
            edit_save::write_sidecar(&cfg, &path, &base, &Doc::new(40, 30), Some(&base), false);
            let (b, d) = annotate::load_editable(&path).unwrap().expect("sidecar");
            assert_eq!(
                (b.width, b.height),
                (40, 30),
                "keep_editable {keep_editable}"
            );
            assert!(d.shapes.is_empty());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_collapses_and_truncates() {
        assert_eq!(preview("a  b\n c", 20), "a b c");
        assert_eq!(preview("abcdefghij", 5), "abcde...");
        assert_eq!(preview("short", 5), "short");
    }
}

#[cfg(test)]
mod ux_tests {
    use super::*;

    #[test]
    fn busy_clipboard_reads_as_plain_words() {
        let e = anyhow::anyhow!("OpenClipboard failed: Access is denied. (0x80070005)");
        let m = clipboard_error(&e);
        assert!(m.starts_with("Another app is using the clipboard"));
        assert!(!m.contains("0x8007"));
        assert!(!clipboard_text_error(&e).contains("OpenClipboard"));
        // Other failures never show the raw chain.
        let other = anyhow::anyhow!("item no longer exists");
        assert!(!clipboard_text_error(&other).contains("no longer exists"));
        assert!(item_copy_error(&other).contains("moved or deleted"));
        let gone = anyhow::Error::new(std::io::Error::from_raw_os_error(2));
        assert!(!item_copy_error(&gone).contains("os error"));
        // A refused clipboard is not an unreadable file.
        let refused = anyhow::anyhow!("GlobalLock failed").context(CLIPBOARD_MARK);
        let m = item_copy_error(&refused);
        assert!(m.contains("clipboard"), "{m}");
        assert!(!m.contains("could not be read"), "{m}");
    }
}
