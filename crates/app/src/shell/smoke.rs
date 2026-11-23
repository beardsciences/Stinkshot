//! `stinkshot --smoke-test` (hidden): start-up check for CI on systems without a desktop
//! session (Linux under Xvfb). Runs the real start-up path as far as is safe off Windows:
//! config, language, fonts and history from a fresh temporary profile, the wgpu device
//! (`gpu.rs`), eframe with the hidden root viewport, and the Settings window as an
//! immediate viewport like the shell shows it. After `FRAMES` frames painted in the
//! Settings window it closes and exits 0. Any failure, panic or a run longer than `TIMEOUT`
//! exits 1 with a message on stderr.
//!
//! Left out on purpose: the single-instance lock, IPC, tray, hotkeys, PrintScreen takeover,
//! autostart, capture warm-up and background workers. They talk to the desktop or to another
//! instance, and on Windows they would collide with an installed copy.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use stinkshot_core::config::Config;
use stinkshot_core::fonts::Fonts;
use stinkshot_history::History;

use super::{accent_color, resolve_theme, root_viewport, show_surface};
use crate::settings_ui::SettingsWindow;
use crate::state::{Shared, Surface, SurfaceStatus};
use crate::{logger, sysutil, theme};

/// Frames the Settings window must paint, with its native window up, before the test passes.
const FRAMES: u32 = 5;
const TIMEOUT: Duration = Duration::from_secs(20);

/// Run the smoke test and exit the process.
pub fn run() -> ! {
    let started = Instant::now();
    sysutil::attach_parent_console();
    let dir = match isolate() {
        Ok(d) => d,
        Err(e) => fail(&format!("{e:#}")),
    };
    let _ = std::thread::Builder::new()
        .name("smoke-watchdog".into())
        .spawn(|| {
            std::thread::sleep(TIMEOUT);
            fail(&format!("timed out after {} s", TIMEOUT.as_secs()));
        });
    stinkshot_platform::dpi::init();
    stinkshot_platform::keep_mta_alive();
    logger::init();
    eprintln!("smoke: profile in {}", dir.display());
    match std::panic::catch_unwind(start) {
        Ok(Ok(frames)) => {
            eprintln!(
                "smoke: ok, {frames} Settings frames in {:.1} s",
                started.elapsed().as_secs_f32()
            );
            log::logger().flush();
            // Best effort: the open log file keeps the folder on Windows.
            let _ = std::fs::remove_dir_all(&dir);
            std::process::exit(0);
        }
        Ok(Err(e)) => fail(&format!("{e:#} (log: {})", logger::path().display())),
        Err(_) => fail(&format!("panicked (log: {})", logger::path().display())),
    }
}

fn fail(why: &str) -> ! {
    log::error!("smoke test failed: {why}");
    log::logger().flush();
    eprintln!("stinkshot: smoke test failed: {why}");
    std::process::exit(1);
}

/// Point config, log and history at a new temporary folder, under the `smoke` profile.
fn isolate() -> anyhow::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("stinkshot-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    // SAFETY: called from `main` before any other thread exists, so nothing reads the
    // environment concurrently. `Config::dir` and `Config::history_folder` read these first
    // on every system.
    unsafe {
        std::env::set_var("APPDATA", &dir);
        std::env::set_var("USERPROFILE", &dir);
        std::env::set_var("STINKSHOT_PROFILE", "smoke");
    }
    Ok(dir)
}

/// The start-up path; returns the number of Settings frames painted.
fn start() -> anyhow::Result<u32> {
    let gpu = crate::gpu::start();
    let config = Config::load();
    crate::l10n::apply(&config.general.language);
    let fonts = Arc::new(Fonts::load_system().context("load system fonts")?);
    let history_dir = config.history_folder();
    let history = Arc::new(
        History::open(&history_dir)
            .with_context(|| format!("open history folder {}", history_dir.display()))?,
    );
    let wgpu_options = crate::gpu::options(gpu);
    let (dark, accent) = resolve_theme(&config);
    let shared = Shared {
        config,
        config_dirty: false,
        history,
        fonts,
        ocr: None,
        outbox: Vec::new(),
        dark,
        accent,
        update: Default::default(),
        update_check_failed: false,
    };
    let options = eframe::NativeOptions {
        viewport: root_viewport(),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        clipboard_shortcuts: false,
        ..Default::default()
    };
    let frames = Arc::new(AtomicU32::new(0));
    let seen = frames.clone();
    eframe::run_native(
        "Stinkshot",
        options,
        Box::new(move |cc| {
            let rs = cc
                .wgpu_render_state
                .as_ref()
                .ok_or("eframe started without a wgpu renderer")?;
            crate::gpu::install_handlers(&rs.device);
            let info = rs.adapter.get_info();
            eprintln!("smoke: adapter {} ({:?})", info.name, info.backend);
            theme::install(&cc.egui_ctx, &shared.fonts, dark, accent_color(accent));
            let settings = SettingsWindow::new(&shared);
            Ok(Box::new(Smoke {
                shared,
                settings,
                frames: seen,
            }))
        }),
    )
    .map_err(|e| anyhow!("eframe: {e}"))?;
    let n = frames.load(Ordering::SeqCst);
    if n < FRAMES {
        return Err(anyhow!(
            "eframe exited after {n} Settings frames, expected {FRAMES}"
        ));
    }
    Ok(n)
}

struct Smoke {
    shared: Shared,
    settings: SettingsWindow,
    frames: Arc<AtomicU32>,
}

impl eframe::App for Smoke {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if crate::gpu::lost() {
            fail("wgpu device lost");
        }
        let title = self.settings.title();
        let id = egui::ViewportId::from_hash_of(&title);
        let builder = self
            .settings
            .viewport_builder(&self.shared)
            .with_title(title)
            .with_icon(crate::app_icon::window_icon());
        let (settings, shared) = (&mut self.settings, &mut self.shared);
        // Counts only frames drawn in a native window of its own (not embedded in the root).
        let painted = ctx.show_viewport_immediate(id, builder, |ctx, class| {
            let status = show_surface(ctx, settings, shared);
            let has_window = ctx
                .input(|i| i.viewport().inner_rect)
                .is_some_and(|r| r.area() > 0.0);
            (
                status,
                has_window && class != egui::ViewportClass::EmbeddedWindow,
            )
        });
        // Nothing is acted on: no settings are saved and no commands run.
        self.shared.outbox.clear();
        self.shared.config_dirty = false;
        match painted {
            (SurfaceStatus::Close, _) => fail("the Settings window closed itself"),
            (SurfaceStatus::Open, true) => {
                let n = self.frames.fetch_add(1, Ordering::SeqCst) + 1;
                if n >= FRAMES {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
            }
            (SurfaceStatus::Open, false) => {}
        }
        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
