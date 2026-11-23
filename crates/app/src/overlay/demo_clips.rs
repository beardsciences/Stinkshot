//! Website demo clips: drives the real overlay and History window offscreen with scripted
//! egui input over a demo desktop image and pipes the frames to ffmpeg. Nothing touches the
//! real screen, cursor or windows: the overlay never gets a window (no HWND, so no styling,
//! topmost or focus calls), element snapping is off, and the "windows" it can highlight are
//! the demo picture's own window rects.
//!
//! Ignored by default (needs a GPU, ffmpeg and the demo inputs):
//! `STINKSHOT_DEMO_DIR=<dir> STINKSHOT_FFMPEG=<ffmpeg.exe> cargo test -p stinkshot demo_clips -- --ignored --nocapture`
//! `<dir>` holds `desktop.png`, `desktop.png.windows.json` and `history/` from
//! `scripts/demo-clips/`; each clip is written as `<dir>/<name>.mkv` (lossless H.264 RGB).
//! `Step::Snap` frames are also saved as `<dir>/stills/<name>.png` (the `stills` set is
//! verification shots only, no clip).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as Proc, Stdio};

use eframe::egui_wgpu::{self, wgpu};
use egui::{Modifiers, PointerButton, Shape as EShape, Stroke, ViewportId, ViewportInfo};
use stinkshot_core::config::Config;
use stinkshot_platform::capture::Frozen;

use super::*;
use crate::state::OverlayMode;

const FPS: f32 = 30.0;
/// Physical px per egui point of the script's coordinates (a 150 % display). Scripts are
/// written in these 1920x1200 px; `STINKSHOT_DEMO_SCALE` renders everything larger.
const PPP: f32 = 1.5;

/// Render scale over the script coordinates (1.5 renders the 1920x1200 desktop at 2880x1800).
fn scale() -> f32 {
    std::env::var("STINKSHOT_DEMO_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0)
}

/// Physical px per point of the rendered frames.
fn ppp() -> f32 {
    PPP * scale()
}
const ACCENT: [u8; 4] = [0x7f, 0xd3, 0x6a, 0xff];

/// One scripted step. Positions are physical px of the rendered frame.
#[derive(Clone)]
enum Step {
    /// Glide the pointer to a spot (ease in-out).
    Move(f32, f32, f32),
    Down,
    Up,
    /// Press-glide-release.
    Drag(f32, f32, f32),
    Key(Key, Modifiers),
    /// Type text one character per `secs`.
    Type(&'static str, f32),
    Wait(f32),
    /// Frames before this are rendered but not recorded (setup).
    Record,
    /// Secondary (right) click where the pointer is.
    RightClick,
    /// Modifiers held from here on (Shift for multi-region drags).
    Hold(Modifiers),
    /// Save this frame as `<dir>/stills/<name>.png`.
    Snap(&'static str),
}

/// One frame's input.
struct Frame {
    events: Vec<Event>,
    pos: Pos2,
    modifiers: Modifiers,
    snap: Option<&'static str>,
}

/// Per-frame input events built from a script.
struct Timeline {
    frames: Vec<Frame>,
    record_from: Option<usize>,
}

fn ease(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn timeline(start: (f32, f32), steps: &[Step]) -> Timeline {
    let mut frames: Vec<Frame> = Vec::new();
    let mut pos = Pos2::new(start.0 / PPP, start.1 / PPP);
    let mut mods = Modifiers::NONE;
    let mut record_from = None;
    let mut pending: Vec<Event> = vec![Event::PointerMoved(pos)];
    let push = |frames: &mut Vec<Frame>, ev: &mut Vec<Event>, p: Pos2, m: Modifiers| {
        frames.push(Frame {
            events: std::mem::take(ev),
            pos: p,
            modifiers: m,
            snap: None,
        });
    };
    let glide = |frames: &mut Vec<Frame>,
                 ev: &mut Vec<Event>,
                 pos: &mut Pos2,
                 to: (f32, f32),
                 secs: f32,
                 m: Modifiers| {
        let from = *pos;
        let to = Pos2::new(to.0 / PPP, to.1 / PPP);
        let n = (secs * FPS).round().max(1.0) as usize;
        for i in 1..=n {
            let t = ease(i as f32 / n as f32);
            *pos = from + (to - from) * t;
            ev.push(Event::PointerMoved(*pos));
            push(frames, ev, *pos, m);
        }
    };
    let button = |p: Pos2, b: PointerButton, pressed: bool, m: Modifiers| Event::PointerButton {
        pos: p,
        button: b,
        pressed,
        modifiers: m,
    };
    let primary = PointerButton::Primary;
    for s in steps {
        match s.clone() {
            Step::Move(x, y, secs) => {
                glide(&mut frames, &mut pending, &mut pos, (x, y), secs, mods)
            }
            Step::Down => {
                pending.push(button(pos, primary, true, mods));
                push(&mut frames, &mut pending, pos, mods);
            }
            Step::Up => {
                pending.push(button(pos, primary, false, mods));
                push(&mut frames, &mut pending, pos, mods);
            }
            Step::RightClick => {
                for pressed in [true, false] {
                    pending.push(button(pos, PointerButton::Secondary, pressed, mods));
                    push(&mut frames, &mut pending, pos, mods);
                }
            }
            Step::Drag(x, y, secs) => {
                pending.push(button(pos, primary, true, mods));
                push(&mut frames, &mut pending, pos, mods);
                glide(&mut frames, &mut pending, &mut pos, (x, y), secs, mods);
                pending.push(button(pos, primary, false, mods));
                push(&mut frames, &mut pending, pos, mods);
            }
            Step::Key(key, modifiers) => {
                for pressed in [true, false] {
                    pending.push(Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed,
                        repeat: false,
                        modifiers,
                    });
                }
                push(&mut frames, &mut pending, pos, modifiers);
            }
            Step::Type(text, secs) => {
                let per = (secs * FPS).round().max(1.0) as usize;
                for ch in text.chars() {
                    pending.push(Event::Text(ch.to_string()));
                    push(&mut frames, &mut pending, pos, mods);
                    for _ in 1..per {
                        push(&mut frames, &mut pending, pos, mods);
                    }
                }
            }
            Step::Wait(secs) => {
                for _ in 0..(secs * FPS).round() as usize {
                    push(&mut frames, &mut pending, pos, mods);
                }
            }
            Step::Record => record_from = Some(frames.len()),
            Step::Hold(m) => mods = m,
            Step::Snap(name) => {
                push(&mut frames, &mut pending, pos, mods);
                frames.last_mut().unwrap().snap = Some(name);
            }
        }
    }
    Timeline {
        frames,
        record_from,
    }
}

/// Offscreen wgpu target plus an ffmpeg process that takes raw RGBA frames.
struct Rig {
    dev: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
    tex: wgpu::Texture,
    buf: wgpu::Buffer,
    w: u32,
    h: u32,
    row: u32,
}

/// Non-sRGB like the app's own surfaces (egui-wgpu's `preferred_framebuffer_format`): egui
/// then blends in gamma space as on screen. An sRGB target blends in linear space and
/// renders every translucent fill (dims, discs, shadows) noticeably lighter.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl Rig {
    fn new(dev: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32) -> Rig {
        let renderer = egui_wgpu::Renderer::new(dev, FORMAT, Default::default());
        let tex = dev.create_texture(&wgpu::TextureDescriptor {
            label: Some("demo clip"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let row = (w * 4).div_ceil(256) * 256;
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demo clip readback"),
            size: u64::from(row * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Rig {
            dev: dev.clone(),
            queue: queue.clone(),
            renderer,
            tex,
            buf,
            w,
            h,
            row,
        }
    }

    fn render(&mut self, ctx: &Context, mut out: egui::FullOutput) -> Vec<u8> {
        let (dev, queue) = (&self.dev, &self.queue);
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                self.renderer.update_texture(dev, queue, *id, delta);
            }
        }
        let jobs = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        let view = self.tex.create_view(&Default::default());
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.w, self.h],
            pixels_per_point: out.pixels_per_point,
        };
        let mut enc = dev.create_command_encoder(&Default::default());
        let mut cmds = self
            .renderer
            .update_buffers(dev, queue, &mut enc, &jobs, &screen);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("demo clip"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.11,
                                g: 0.11,
                                b: 0.12,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.w,
                height: self.h,
                depth_or_array_layers: 1,
            },
        );
        cmds.push(enc.finish());
        queue.submit(cmds);
        for id in &out.textures_delta.free {
            self.renderer.free_texture(id);
        }
        out.textures_delta.clear();
        let slice = self.buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        dev.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let mut px = Vec::with_capacity((self.w * self.h * 4) as usize);
        {
            let data = slice.get_mapped_range().expect("mapped");
            for y in 0..self.h {
                let s = (y * self.row) as usize;
                px.extend_from_slice(&data[s..s + (self.w * 4) as usize]);
            }
        }
        self.buf.unmap();
        px
    }
}

fn ffmpeg(out: &Path, w: u32, h: u32) -> Child {
    let exe = std::env::var("STINKSHOT_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    Proc::new(exe)
        .args(["-y", "-hide_banner", "-loglevel", "error", "-f", "rawvideo"])
        .args([
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{w}x{h}"),
            "-r",
            "30",
            "-i",
            "-",
        ])
        .args(["-c:v", "libx264rgb", "-qp", "0", "-preset", "ultrafast"])
        .arg(out)
        .stdin(Stdio::piped())
        .spawn()
        .expect("start ffmpeg (set STINKSHOT_FFMPEG)")
}

/// The pointer, drawn on top in the shape the surface asked for (the real OS cursor is
/// never part of an offscreen render).
fn paint_cursor(ctx: &Context, p: Pos2, icon: CursorIcon) {
    let painter = ctx.layer_painter(LayerId::new(Order::Debug, egui::Id::unique("demo-cursor")));
    let black = Color32::from_rgb(10, 10, 10);
    let white = Color32::WHITE;
    match icon {
        CursorIcon::Crosshair => {
            for (w, c) in [(3.0, white), (1.0, black)] {
                let s = Stroke::new(w, c);
                painter.line_segment([p - Vec2::new(10.0, 0.0), p + Vec2::new(10.0, 0.0)], s);
                painter.line_segment([p - Vec2::new(0.0, 10.0), p + Vec2::new(0.0, 10.0)], s);
            }
        }
        CursorIcon::Text => {
            for (w, c) in [(3.0, white), (1.0, black)] {
                let s = Stroke::new(w, c);
                painter.line_segment([p - Vec2::new(0.0, 9.0), p + Vec2::new(0.0, 9.0)], s);
                painter.line_segment([p + Vec2::new(-3.5, -9.0), p + Vec2::new(3.5, -9.0)], s);
                painter.line_segment([p + Vec2::new(-3.5, 9.0), p + Vec2::new(3.5, 9.0)], s);
            }
        }
        _ => {
            let pts = [
                (0.0, 0.0),
                (0.0, 16.0),
                (4.0, 12.5),
                (6.8, 18.5),
                (9.2, 17.4),
                (6.6, 11.6),
                (11.6, 11.6),
            ]
            .map(|(x, y)| p + Vec2::new(x, y));
            painter.add(EShape::convex_polygon(
                vec![pts[0], pts[1], pts[2], pts[5], pts[6]],
                black,
                Stroke::NONE,
            ));
            painter.add(EShape::convex_polygon(
                vec![pts[2], pts[3], pts[4], pts[5]],
                black,
                Stroke::NONE,
            ));
            painter.add(EShape::closed_line(pts.to_vec(), Stroke::new(1.2, white)));
        }
    }
}

fn input(size_pt: Vec2, t: f64, events: Vec<Event>, modifiers: Modifiers) -> egui::RawInput {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size_pt)),
        time: Some(t),
        predicted_dt: 1.0 / FPS,
        events: std::iter::once(Event::ModifiersChanged(modifiers))
            .chain(events)
            .collect(),
        focused: true,
        ..Default::default()
    };
    raw.viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            native_pixels_per_point: Some(ppp()),
            focused: Some(true),
            inner_rect: Some(Rect::from_min_size(Pos2::ZERO, size_pt)),
            ..Default::default()
        },
    );
    raw
}

/// Runs a script against `show`, recording frames from `Step::Record` on to `out` (no
/// `Record`: nothing is encoded, only the `Snap` stills are written). Paced in real time, so
/// timers in the surfaces (caret blink, debounce, background renders) behave as they would
/// live.
fn run_clip(
    rig: &mut Rig,
    shared: &mut Shared,
    out: &Path,
    start: (f32, f32),
    steps: &[Step],
    mut show: impl FnMut(&Context, &mut Shared),
) {
    let ctx = Context::default();
    theme::install(&ctx, &shared.fonts, shared.dark, color32(shared.accent));
    let size_pt = Vec2::new(rig.w as f32 / ppp(), rig.h as f32 / ppp());
    let tl = timeline(start, steps);
    let mut enc = tl.record_from.map(|_| ffmpeg(out, rig.w, rig.h));
    let stills = out.parent().unwrap().join("stills");
    let mut icon = CursorIcon::Default;
    let t0 = Instant::now();
    let frame = Duration::from_secs_f32(1.0 / FPS);
    for (i, f) in tl.frames.into_iter().enumerate() {
        let due = t0 + frame * i as u32;
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        let raw = input(size_pt, i as f64 / f64::from(FPS), f.events, f.modifiers);
        let pos = f.pos;
        let out = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            show(&ctx, shared);
            paint_cursor(&ctx, pos, icon);
        });
        icon = out.platform_output.cursor_icon;
        let px = rig.render(&ctx, out);
        if let (Some(enc), Some(from)) = (enc.as_mut(), tl.record_from)
            && i >= from
        {
            enc.stdin
                .as_mut()
                .unwrap()
                .write_all(&px)
                .expect("ffmpeg stdin");
        }
        if let Some(name) = f.snap {
            std::fs::create_dir_all(&stills).unwrap();
            image::RgbaImage::from_raw(rig.w, rig.h, px)
                .unwrap()
                .save(stills.join(format!("{name}.png")))
                .expect("save still");
        }
        shared.outbox.clear();
    }
    if let Some(mut enc) = enc {
        drop(enc.stdin.take());
        assert!(
            enc.wait().unwrap().success(),
            "ffmpeg failed for {}",
            out.display()
        );
    }
}

fn color32(c: [u8; 4]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn load_rgba(path: &Path) -> Rgba {
    let img = image::open(path).expect("demo image").to_rgba8();
    Rgba {
        width: img.width(),
        height: img.height(),
        data: img.into_raw(),
    }
}

fn demo_windows(path: &Path) -> Vec<WinInfo> {
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("windows.json")).unwrap();
    json.as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let r: Vec<i32> = w["rect"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
            WinInfo {
                // Never a real window: element snapping is off, so these are only compared.
                hwnd: -(i as isize) - 100,
                rect: PxRect::new(r[0], r[1], r[2], r[3]),
                title: w["title"].as_str().unwrap().to_string(),
                process: "demo".into(),
                pid: 0,
                maximized: false,
            }
        })
        .collect()
}

fn shared_for(history: &Path, fonts: Arc<Fonts>) -> Shared {
    let mut config = Config::default();
    config.capture.snap_to_elements = false;
    Shared {
        config,
        config_dirty: false,
        history: Arc::new(stinkshot_history::History::open(history).expect("history")),
        fonts,
        ocr: None,
        outbox: Vec::new(),
        dark: true,
        accent: ACCENT,
        update: Default::default(),
        update_check_failed: false,
    }
}

fn overlay(desktop: &Rgba, windows: &[WinInfo], cursor: (i32, i32), shared: &Shared) -> Overlay {
    overlay_in(OverlayMode::Capture, desktop, windows, cursor, shared)
}

fn overlay_in(
    mode: OverlayMode,
    desktop: &Rgba,
    windows: &[WinInfo],
    cursor: (i32, i32),
    shared: &Shared,
) -> Overlay {
    let req = OverlayRequest {
        mode,
        frozen: Frozen {
            bounds: PxRect::new(0, 0, desktop.width as i32, desktop.height as i32),
            image: desktop.clone(),
            cursor: PxPoint {
                x: (cursor.0 as f32 * scale()) as i32,
                y: (cursor.1 as f32 * scale()) as i32,
            },
            cursor_shot: None,
            cursor_drawn: false,
        },
        windows: windows.to_vec(),
        layers: Vec::new(),
        notice: None,
        from_tray_click: false,
    };
    let mut o = Overlay::new(req, shared);
    // One demo monitor at 150 %, not the machine's real layout.
    o.monitors = vec![(o.local_bounds(), ppp())];
    o.create_scale = ppp();
    o.bar_origin = None;
    o.snapper = None;
    o
}

fn key(k: Key) -> Step {
    Step::Key(k, Modifiers::NONE)
}

#[test]
#[ignore = "needs a GPU, ffmpeg and the demo inputs; see the module docs"]
fn demo_clips() {
    let dir = PathBuf::from(std::env::var("STINKSHOT_DEMO_DIR").expect("STINKSHOT_DEMO_DIR"));
    let only = std::env::var("STINKSHOT_DEMO_ONLY").unwrap_or_default();
    // STINKSHOT_DEMO_HOME: the user folder the pages show (Settings prints the config and
    // History paths), so published pictures carry a neutral path, not this PC's user name.
    // Only read here; nothing in the harness writes there.
    if let Ok(home) = std::env::var("STINKSHOT_DEMO_HOME") {
        let home = PathBuf::from(home);
        // SAFETY: set before any other thread starts (the GPU thread is spawned below).
        unsafe {
            std::env::set_var("USERPROFILE", &home);
            std::env::set_var("APPDATA", home.join("AppData").join("Roaming"));
            std::env::set_var("LOCALAPPDATA", home.join("AppData").join("Local"));
            std::env::remove_var("STINKSHOT_PROFILE");
        }
    }
    // STINKSHOT_DEMO_LANG=es checks a translation for clipping (site media stays English).
    crate::l10n::apply(&std::env::var("STINKSHOT_DEMO_LANG").unwrap_or_else(|_| "en".into()));
    toolbar::remember(None);
    let setup = crate::gpu::start()
        .and_then(|h| h.join().ok().flatten())
        .expect("no wgpu device");
    let fonts = Arc::new(Fonts::load_system().unwrap());
    let mut desktop = load_rgba(&dir.join("desktop.png"));
    let mut windows = demo_windows(&dir.join("desktop.png.windows.json"));
    // Scripts are in 1920x1200 px; a larger render scale scales the desktop under them.
    let k = scale();
    if (k - 1.0).abs() > f32::EPSILON {
        let img = image::RgbaImage::from_raw(desktop.width, desktop.height, desktop.data).unwrap();
        let (w, h) = (
            (img.width() as f32 * k).round() as u32,
            (img.height() as f32 * k).round() as u32,
        );
        let img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3);
        desktop = Rgba {
            width: w,
            height: h,
            data: img.into_raw(),
        };
        let sc = |v: i32| (v as f32 * k).round() as i32;
        for w in &mut windows {
            w.rect = PxRect::new(sc(w.rect.x), sc(w.rect.y), sc(w.rect.w), sc(w.rect.h));
        }
    }
    let empty_history = dir.join("empty-history");
    std::fs::create_dir_all(&empty_history).unwrap();
    // Clips not in the default set (verification shots) only run when named.
    let extra = ["history-probe", "stills", "probe", "site", "shots"];
    let want = |name: &str| {
        if only.is_empty() {
            !extra.contains(&name)
        } else {
            only.split(',').any(|n| n == name)
        }
    };
    let sz = |v: u32| (v as f32 * scale()).round() as u32;

    // Overlay clips render the full 1920x1200 demo desktop.
    let mut rig = Rig::new(&setup.device, &setup.queue, desktop.width, desktop.height);

    if want("select") {
        let mut shared = shared_for(&empty_history, fonts.clone());
        let start = (700.0, 1000.0);
        let mut o = overlay(&desktop, &windows, (700, 1000), &shared);
        let steps = [
            Step::Record,
            Step::Wait(0.4),
            Step::Move(560.0, 420.0, 0.8),
            Step::Wait(0.5),
            Step::Move(1300.0, 380.0, 0.8),
            Step::Wait(0.5),
            Step::Move(882.0, 288.0, 0.6),
            Step::Wait(0.25),
            Step::Drag(1818.0, 842.0, 1.4),
            Step::Wait(0.3),
            Step::Move(1500.0, 990.0, 0.6),
            Step::Wait(1.4),
        ];
        run_clip(
            &mut rig,
            &mut shared,
            &dir.join("select.mkv"),
            start,
            &steps,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    }

    if want("annotate") {
        let mut shared = shared_for(&empty_history, fonts.clone());
        let start = (882.0, 288.0);
        let mut o = overlay(&desktop, &windows, (882, 288), &shared);
        let steps = [
            Step::Wait(0.2),
            Step::Drag(1818.0, 842.0, 0.2),
            Step::Wait(0.3),
            Step::Move(1500.0, 300.0, 0.2),
            Step::Wait(0.3),
            Step::Record,
            Step::Wait(0.2),
            key(Key::A),
            Step::Move(1610.0, 500.0, 0.6),
            Step::Drag(1736.0, 628.0, 0.6),
            Step::Wait(0.3),
            key(Key::N),
            Step::Move(1085.0, 375.0, 0.6),
            Step::Down,
            Step::Up,
            Step::Wait(0.15),
            Step::Move(1315.0, 375.0, 0.45),
            Step::Down,
            Step::Up,
            Step::Wait(0.15),
            Step::Move(1545.0, 375.0, 0.45),
            Step::Down,
            Step::Up,
            Step::Wait(0.3),
            key(Key::T),
            Step::Move(1300.0, 478.0, 0.5),
            Step::Down,
            Step::Up,
            Step::Wait(0.2),
            Step::Type("Fastest week yet", 0.06),
            Step::Wait(0.4),
            key(Key::Escape),
            Step::Wait(0.2),
            key(Key::Escape),
            Step::Move(1300.0, 980.0, 0.5),
            Step::Wait(1.0),
        ];
        run_clip(
            &mut rig,
            &mut shared,
            &dir.join("annotate.mkv"),
            start,
            &steps,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    }

    if want("redact") {
        let mut shared = shared_for(&empty_history, fonts.clone());
        let start = (150.0, 730.0);
        let mut o = overlay(&desktop, &windows, (150, 730), &shared);
        let steps = [
            Step::Wait(0.2),
            Step::Drag(790.0, 1050.0, 0.2),
            Step::Wait(0.3),
            Step::Move(900.0, 1000.0, 0.2),
            Step::Wait(0.3),
            Step::Record,
            Step::Wait(0.4),
            key(Key::B),
            Step::Move(280.0, 852.0, 0.6),
            Step::Drag(530.0, 880.0, 0.6),
            Step::Wait(0.3),
            Step::Move(280.0, 896.0, 0.4),
            // The demo token runs to about x 590.
            Step::Drag(600.0, 924.0, 0.6),
            Step::Wait(0.3),
            key(Key::Escape),
            Step::Move(1000.0, 1000.0, 0.6),
            Step::Wait(1.6),
        ];
        run_clip(
            &mut rig,
            &mut shared,
            &dir.join("redact.mkv"),
            start,
            &steps,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    }

    if want("history") || want("history-probe") {
        // The History window at its default 1280x820 pt.
        let mut hrig = Rig::new(&setup.device, &setup.queue, sz(1920), sz(1230));
        let mut shared = shared_for(&dir.join("history"), fonts.clone());
        // Windows OCR, local; with an engine present History searches the text in captures.
        shared.ocr = stinkshot_platform::ocr::WindowsOcr::new()
            .ok()
            .map(|o| Arc::new(o) as _);
        assert!(shared.ocr.is_some(), "no OCR language installed");
        let mut w = crate::history_ui::HistoryWindow::new(&shared);
        let probe = want("history-probe");
        let sx: f32 = std::env::var("STINKSHOT_DEMO_SEARCH_X")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(560.0);
        let sy: f32 = std::env::var("STINKSHOT_DEMO_SEARCH_Y")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(66.0);
        let steps: Vec<Step> = if probe {
            vec![Step::Wait(2.0), Step::Record, Step::Wait(0.1)]
        } else {
            vec![
                Step::Wait(2.0),
                Step::Record,
                Step::Wait(0.5),
                Step::Move(sx, sy, 0.8),
                Step::Down,
                Step::Up,
                Step::Wait(0.3),
                Step::Type("cache", 0.12),
                Step::Wait(1.0),
                Step::Type(" hits", 0.12),
                Step::Wait(0.4),
                Step::Move(700.0, 400.0, 0.7),
                Step::Wait(1.6),
            ]
        };
        let name = if probe {
            "history-probe.mkv"
        } else {
            "history.mkv"
        };
        run_clip(
            &mut hrig,
            &mut shared,
            &dir.join(name),
            (1500.0, 900.0),
            &steps,
            |ctx, sh| {
                let _ = crate::state::Surface::show(&mut w, ctx, sh);
            },
        );
    }

    // Positions: `STINKSHOT_DEMO_PTS` holds `name=x,y;...` overrides in physical px of the
    // surface (tuning aid; the defaults below are the published takes).
    let pts_env = std::env::var("STINKSHOT_DEMO_PTS").unwrap_or_default();
    let pt = |name: &str, x: f32, y: f32| -> (f32, f32) {
        for kv in pts_env.split(';') {
            if let Some((k, v)) = kv.split_once('=')
                && k == name
            {
                let mut it = v.split(',').map(|n| n.parse::<f32>().unwrap());
                return (it.next().unwrap(), it.next().unwrap());
            }
        }
        (x, y)
    };
    // Overlay clips that start from a region already selected (not recorded), like
    // `annotate`. `setup` adjusts the shared state first (recent picks, config).
    let region_clip = |rig: &mut Rig,
                       name: &str,
                       region: [(f32, f32); 2],
                       steps: &[Step],
                       setup: &dyn Fn(&mut Shared)| {
        let mut shared = shared_for(&empty_history, fonts.clone());
        setup(&mut shared);
        let (a, b) = (region[0], region[1]);
        let mut o = overlay(&desktop, &windows, (a.0 as i32, a.1 as i32), &shared);
        let mut all = vec![
            Step::Wait(0.2),
            Step::Drag(b.0, b.1, 0.2),
            Step::Wait(0.3),
            Step::Move(b.0 - 300.0, a.1 + 30.0, 0.2),
            Step::Wait(0.3),
            Step::Record,
        ];
        all.extend_from_slice(steps);
        run_clip(
            rig,
            &mut shared,
            &dir.join(format!("{name}.mkv")),
            a,
            &all,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    };
    let none = |_: &mut Shared| {};
    let dash = [(882.0, 288.0), (1818.0, 842.0)];

    if want("steps") {
        // Step counters on the weekly bars, then a callout whose tail is pulled onto W12.
        let br = || key(Key::CloseBracket);
        let steps = [
            Step::Wait(0.3),
            key(Key::N),
            br(),
            br(),
            br(),
            br(),
            Step::Move(981.0, 515.0, 0.4),
            Step::Down,
            Step::Up,
            Step::Wait(0.15),
            Step::Move(1467.0, 580.0, 0.4),
            Step::Down,
            Step::Up,
            Step::Wait(0.15),
            Step::Move(1744.0, 608.0, 0.4),
            Step::Down,
            Step::Up,
            Step::Wait(0.35),
            key(Key::O),
            Step::Move(1430.0, 470.0, 0.5),
            Step::Drag(1680.0, 518.0, 0.5),
            Step::Wait(0.2),
            Step::Type("Fastest week yet", 0.05),
            Step::Wait(0.3),
            key(Key::Escape),
            Step::Wait(0.2),
            key(Key::V),
            Step::Move(1492.0, 547.0, 0.4),
            Step::Wait(0.2),
            Step::Drag(1720.0, 600.0, 0.5),
            Step::Wait(0.3),
            key(Key::Escape),
            Step::Move(1300.0, 980.0, 0.5),
            Step::Wait(0.8),
        ];
        region_clip(&mut rig, "steps", dash, &steps, &none);
    }

    if want("magnify") {
        // Magnifier (square source with Shift) on the Cache Hits card, then a spotlight on
        // the latest week.
        let steps = [
            Step::Wait(0.3),
            key(Key::M),
            // A 90 px square centred on the "Cache Hits" label and the 87% under it.
            Step::Move(1357.0, 350.0, 0.6),
            Step::Hold(Modifiers::SHIFT),
            Step::Drag(1447.0, 440.0, 0.6),
            Step::Hold(Modifiers::NONE),
            Step::Wait(0.2),
            Step::Move(1210.0, 600.0, 0.6),
            Step::Down,
            Step::Up,
            Step::Wait(0.6),
            key(Key::S),
            // The W12 bar and its label (x 1725-1764, y 634-803) with 14 px on every side.
            Step::Move(1711.0, 620.0, 0.6),
            Step::Drag(1778.0, 817.0, 0.6),
            Step::Wait(0.3),
            key(Key::Escape),
            Step::Move(1500.0, 980.0, 0.6),
            Step::Wait(1.4),
        ];
        region_clip(&mut rig, "magnify", dash, &steps, &none);
    }

    if want("measure") {
        // Ruler across a KPI card, then the height of the W1 bar.
        let steps = [
            Step::Wait(0.3),
            key(Key::U),
            Step::Move(898.0, 422.0, 0.6),
            Step::Drag(1110.0, 422.0, 0.9),
            Step::Wait(0.9),
            Step::Move(1000.0, 540.0, 0.6),
            Step::Drag(1000.0, 782.0, 0.9),
            Step::Wait(0.9),
            Step::Move(970.0, 782.0, 0.5),
            Step::Drag(1765.0, 634.0, 1.0),
            Step::Wait(1.2),
        ];
        region_clip(&mut rig, "measure", dash, &steps, &none);
    }

    if want("beautify") {
        // Beautify panel on the code window: turn it on, try backgrounds and an aspect.
        let code = [(70.0, 60.0), (1110.0, 710.0)];
        let p = |k: &str, x: f32, y: f32| {
            let (x, y) = pt(k, x, y);
            [Step::Move(x, y, 0.4), Step::Down, Step::Up, Step::Wait(0.6)]
        };
        let mut steps = vec![Step::Wait(0.3), key(Key::F), Step::Wait(0.6)];
        steps.extend(p("b-on", 1766.0, 96.0));
        steps.extend(p("b-swatch2", 1384.0, 512.0));
        steps.extend(p("b-blurred", 1544.0, 455.0));
        steps.extend(p("b-169", 1417.0, 780.0));
        steps.extend(p("b-gradient", 1366.0, 455.0));
        steps.push(Step::Wait(0.6));
        region_clip(&mut rig, "beautify", code, &steps, &none);
    }

    if want("sensitive") {
        // Find Sensitive Info on the chat: OCR finds the email and the token, Enter redacts.
        let chat = [(150.0, 730.0), (790.0, 1050.0)];
        let steps = [
            Step::Wait(0.4),
            Step::Key(Key::R, Modifiers::COMMAND),
            Step::Wait(2.2),
            key(Key::Enter),
            Step::Wait(0.6),
            Step::Move(1000.0, 1000.0, 0.6),
            Step::Wait(1.6),
        ];
        region_clip(&mut rig, "sensitive", chat, &steps, &|sh: &mut Shared| {
            sh.ocr = stinkshot_platform::ocr::WindowsOcr::new()
                .ok()
                .map(|o| Arc::new(o) as _);
        });
    }

    if want("picker") {
        // Standalone color picker: the loupe follows the pointer, Tab cycles Hex / RGB / HSL.
        // Nothing is clicked (a click would copy to the real clipboard).
        let mut shared = shared_for(&empty_history, fonts.clone());
        shared.config.editor.picker_recent =
            ["#7FD36A", "#4FAF5A", "#E8794A", "#F2C14E", "#9AB4F0"]
                .map(String::from)
                .to_vec();
        let start = (1300.0, 900.0);
        let mut o = overlay_in(
            OverlayMode::SelectRegion(crate::state::RegionPurpose::PickColor),
            &desktop,
            &windows,
            (1300, 900),
            &shared,
        );
        let steps = [
            Step::Record,
            Step::Wait(0.3),
            Step::Move(1744.0, 700.0, 1.0),
            Step::Wait(0.6),
            key(Key::Tab),
            Step::Wait(0.6),
            Step::Move(1050.0, 700.0, 0.9),
            Step::Wait(0.5),
            key(Key::Tab),
            Step::Wait(0.6),
            Step::Move(1540.0, 925.0, 0.9),
            Step::Wait(0.6),
            key(Key::Tab),
            Step::Wait(0.5),
            Step::Move(1300.0, 900.0, 0.6),
            Step::Wait(0.6),
        ];
        run_clip(
            &mut rig,
            &mut shared,
            &dir.join("picker.mkv"),
            start,
            &steps,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    }

    if want("multi") {
        // Multi-region: a region, two more with Shift+drag, then a tool combines them.
        let mut shared = shared_for(&empty_history, fonts.clone());
        let start = (700.0, 1000.0);
        let mut o = overlay(&desktop, &windows, (700, 1000), &shared);
        let steps = [
            Step::Record,
            Step::Wait(0.3),
            Step::Move(897.0, 360.0, 0.7),
            Step::Drag(1111.0, 434.0, 0.6),
            Step::Wait(0.4),
            Step::Move(1356.0, 360.0, 0.5),
            Step::Hold(Modifiers::SHIFT),
            Step::Drag(1571.0, 434.0, 0.6),
            Step::Wait(0.3),
            // From the far corner: the second region's chips sit over the near one.
            Step::Move(1801.0, 434.0, 0.4),
            Step::Drag(1586.0, 360.0, 0.6),
            Step::Hold(Modifiers::NONE),
            Step::Wait(0.6),
            key(Key::A),
            Step::Wait(0.6),
            // One arrow across the combined picture, from Runs back to Median.
            Step::Move(1190.0, 585.0, 0.5),
            Step::Drag(780.0, 585.0, 0.7),
            Step::Move(1300.0, 980.0, 0.6),
            Step::Wait(1.2),
        ];
        run_clip(
            &mut rig,
            &mut shared,
            &dir.join("multi.mkv"),
            start,
            &steps,
            |ctx, sh| {
                o.ensure_base_texture(ctx);
                o.frame(ctx, sh);
            },
        );
    }

    if want("stills") || want("probe") {
        // Verification shots of the popup surfaces (menus, combo dropdowns, tooltips). Each
        // surface gets its own rig at its default size.
        let probe = want("probe");
        let shot = |name: &str,
                    w: u32,
                    h: u32,
                    shared: &mut Shared,
                    steps: Vec<Step>,
                    surface: &mut dyn crate::state::Surface| {
            let mut r = Rig::new(&setup.device, &setup.queue, sz(w), sz(h));
            run_clip(
                &mut r,
                shared,
                &dir.join(format!("{name}.mkv")),
                (w as f32 - 20.0, h as f32 - 20.0),
                &steps,
                |ctx, sh| {
                    let _ = surface.show(ctx, sh);
                },
            );
        };
        let hover = |name: &'static str, at: (f32, f32)| {
            vec![
                Step::Move(at.0, at.1, 0.3),
                Step::Wait(1.2),
                Step::Snap(name),
            ]
        };
        let menu = |name: &'static str, at: (f32, f32), row: (f32, f32)| {
            vec![
                Step::Move(at.0, at.1, 0.3),
                Step::Wait(0.3),
                Step::RightClick,
                Step::Wait(0.5),
                Step::Snap(name),
                Step::Move(at.0 + row.0, at.1 + row.1, 0.3),
                Step::Wait(0.5),
                Step::Snap(Box::leak(format!("{name}-hover").into_boxed_str())),
                Step::Key(Key::Escape, Modifiers::NONE),
                Step::Wait(0.3),
            ]
        };
        let click_open = |name: &'static str, at: (f32, f32), row: (f32, f32)| {
            vec![
                Step::Move(at.0, at.1, 0.3),
                Step::Down,
                Step::Up,
                Step::Wait(0.5),
                Step::Snap(name),
                Step::Move(at.0 + row.0, at.1 + row.1, 0.3),
                Step::Wait(0.5),
                Step::Snap(Box::leak(format!("{name}-hover").into_boxed_str())),
                Step::Key(Key::Escape, Modifiers::NONE),
                Step::Wait(0.3),
            ]
        };

        // History.
        let mut shared = shared_for(&dir.join("history"), fonts.clone());
        shared.ocr = stinkshot_platform::ocr::WindowsOcr::new()
            .ok()
            .map(|o| Arc::new(o) as _);
        let mut hw = crate::history_ui::HistoryWindow::new(&shared);
        let mut steps = vec![Step::Wait(2.0), Step::Snap("history-idle")];
        if !probe {
            let un = pt("unstarred", 1140.0, 300.0);
            let st = pt("starred", 634.0, 600.0);
            steps.extend(hover("history-hover-unstarred", un));
            steps.extend(hover(
                "history-trash-tip",
                pt("trash", un.0 + 150.0, un.1 - 112.0),
            ));
            steps.extend(hover("history-hover-starred", st));
            steps.extend(menu("history-menu", un, (90.0, 60.0)));
            steps.extend(click_open(
                "history-date",
                pt("date", 1590.0, 66.0),
                (-40.0, 110.0),
            ));
        }
        shot("stills-history", 1920, 1230, &mut shared, steps, &mut hw);

        // Pin of the dashboard window.
        let r = windows
            .iter()
            .find(|w| w.title.starts_with("Build Times"))
            .unwrap()
            .rect;
        let img = image::RgbaImage::from_raw(desktop.width, desktop.height, desktop.data.clone())
            .unwrap();
        let crop = image::imageops::crop_imm(&img, r.x as u32, r.y as u32, r.w as u32, r.h as u32)
            .to_image();
        let (pw, ph) = (crop.width(), crop.height());
        let pin_img = Rgba {
            width: pw,
            height: ph,
            data: crop.into_raw(),
        };
        let mut pin = crate::pin::Pin::new(
            pin_img,
            "Build Times".into(),
            None,
            Some(r),
            1,
            crate::pin::PinOpts {
                task_view: false,
                always_toolbar: false,
            },
        );
        let mut steps = vec![Step::Wait(0.5), Step::Snap("pin-idle")];
        if !probe {
            steps.extend(hover("pin-tip", pt("pin-tool", 897.0, 35.0)));
            steps.extend(menu("pin-menu", (400.0, 300.0), (60.0, 60.0)));
        }
        shot("stills-pin", pw, ph, &mut shared, steps, &mut pin);

        // Settings: General (theme and language dropdowns), then Recording (its dropdown rows).
        let mut shared = shared_for(&empty_history, fonts.clone());
        let mut sw = crate::settings_ui::SettingsWindow::new(&shared);
        let mut steps = vec![Step::Wait(1.0), Step::Snap("settings-general")];
        if !probe {
            steps.extend(click_open(
                "settings-theme",
                pt("theme", 1338.0, 274.0),
                (-60.0, 70.0),
            ));
            steps.extend(click_open(
                "settings-language",
                pt("language", 1338.0, 386.0),
                (-60.0, 70.0),
            ));
            steps.extend(hover("settings-tip", pt("tip", 1111.0, 496.0)));
        }
        shot("stills-settings", 1560, 1140, &mut shared, steps, &mut sw);
        sw.show_section(crate::settings_ui::Section::Recording);
        let mut steps = vec![Step::Wait(1.0), Step::Snap("settings-recording")];
        if !probe {
            steps.extend(click_open(
                "settings-recording-combo",
                pt("reccombo", 1352.0, 225.0),
                (-60.0, 70.0),
            ));
        }
        shot("stills-settings2", 1560, 1140, &mut shared, steps, &mut sw);

        // Capture toast (thumbnail and action buttons with tooltips).
        let thumb = load_rgba(
            &dir.join("history").join(
                std::fs::read_dir(dir.join("history"))
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name())
                    .find(|n| n.to_string_lossy().ends_with(".png"))
                    .unwrap(),
            ),
        );
        let kind = crate::toast::ToastKind::Capture {
            thumb,
            title: "Copied to Clipboard".into(),
            body: "1600 x 1000, saved to History".into(),
            // Any id: the actions are only hovered, never clicked.
            item: Some(1),
            can_edit: true,
        };
        let size = crate::toast::logical_size(&kind);
        let mut toast = crate::toast::Toast::new(kind, 7, 0);
        let (tw, th) = ((size.x * PPP).round() as u32, (size.y * PPP).round() as u32);
        let mut steps = vec![Step::Wait(1.0), Step::Snap("toast-idle")];
        if !probe {
            steps.extend(hover("toast-tip", pt("toast-btn", 300.0, 150.0)));
        }
        shot("stills-toast", tw, th, &mut shared, steps, &mut toast);
    }

    if want("site") {
        // Stills the website shows as pictures: History and two Settings pages, in both
        // themes, with default settings. The pointer starts off the canvas so it is not drawn.
        let still = |name: String,
                     w: u32,
                     h: u32,
                     shared: &mut Shared,
                     surface: &mut dyn crate::state::Surface| {
            let name: &'static str = Box::leak(name.into_boxed_str());
            let mut r = Rig::new(&setup.device, &setup.queue, sz(w), sz(h));
            run_clip(
                &mut r,
                shared,
                &dir.join(format!("{name}.mkv")),
                (-200.0, -200.0),
                &[Step::Wait(2.0), Step::Snap(name)],
                |ctx, sh| {
                    let _ = surface.show(ctx, sh);
                },
            );
        };
        for (theme, dark) in [("dark", true), ("light", false)] {
            let mut shared = shared_for(&dir.join("history"), fonts.clone());
            shared.dark = dark;
            shared.ocr = stinkshot_platform::ocr::WindowsOcr::new()
                .ok()
                .map(|o| Arc::new(o) as _);
            let mut hw = crate::history_ui::HistoryWindow::new(&shared);
            still(
                format!("site-history-{theme}"),
                1920,
                1230,
                &mut shared,
                &mut hw,
            );

            let mut shared = shared_for(&empty_history, fonts.clone());
            shared.dark = dark;
            // The site shows the app as installed: element snapping is on by default.
            shared.config = Config::default();
            let mut sw = crate::settings_ui::SettingsWindow::new(&shared);
            use crate::settings_ui::Section as S;
            let mut pages = vec![
                ("recording", S::Recording),
                ("privacy", S::Privacy),
                ("general", S::General),
            ];
            // STINKSHOT_DEMO_ALL_PAGES=1: every page, for checking a translation.
            if std::env::var("STINKSHOT_DEMO_ALL_PAGES").as_deref() == Ok("1") {
                pages.extend([
                    ("hotkeys", S::Hotkeys),
                    ("capture", S::Capture),
                    ("editor", S::Editor),
                    ("history", S::History),
                    ("help", S::Help),
                    ("about", S::About),
                ]);
            }
            if std::env::var("STINKSHOT_DEMO_ALL_PAGES").as_deref() == Ok("1") {
                for step in 0..4 {
                    let mut tour = crate::tour::TourWindow::at_step(step);
                    still(
                        format!("tour-{step}-{theme}"),
                        840,
                        600,
                        &mut shared,
                        &mut tour,
                    );
                }
                let summary = crate::l10n::tr!(
                    "shell.hud.mp4",
                    fps = 60,
                    audio = crate::l10n::tr!("shell.hud.mic_system")
                );
                let mut hud = crate::record_ui::RecordHud::new(
                    PxRect::new(0, 0, 1280, 720),
                    summary,
                    crate::record_ui::HudToggles {
                        mic: true,
                        system: true,
                        webcam: false,
                        overlays: true,
                    },
                );
                still(format!("hud-{theme}"), 528, 108, &mut shared, &mut hud);
            }
            for (page, section) in pages {
                sw.show_section(section);
                still(
                    format!("site-settings-{page}-{theme}"),
                    1560,
                    1140,
                    &mut shared,
                    &mut sw,
                );
            }
        }
    }

    if want("shots") {
        // Release screenshots of whole app windows (scripts/site-media.ps1 frames them for the
        // site and the Store): History, a History search, four Settings pages and the editor,
        // in both themes, default settings. Sizes are points; STINKSHOT_DEMO_SCALE sets the
        // pixel density (1.0 is 150 %). STINKSHOT_DEMO_STORE=1 renders the Store build's pages
        // (no update check or Update Now), for the Store screenshots.
        let store = std::env::var("STINKSHOT_DEMO_STORE").as_deref() == Ok("1");
        crate::update::TEST_STORE_BUILD.with(|c| c.set(Some(store)));
        let px = |v: f32| (v * ppp()).round() as u32;
        let shot = |name: String,
                    size_pt: (f32, f32),
                    steps: Vec<Step>,
                    shared: &mut Shared,
                    surface: &mut dyn crate::state::Surface| {
            let name: &'static str = Box::leak(name.into_boxed_str());
            let mut steps = steps;
            steps.push(Step::Snap(name));
            let mut r = Rig::new(&setup.device, &setup.queue, px(size_pt.0), px(size_pt.1));
            run_clip(
                &mut r,
                shared,
                &dir.join(format!("{name}.mkv")),
                (-200.0, -200.0),
                &steps,
                |ctx, sh| {
                    let _ = surface.show(ctx, sh);
                },
            );
        };
        for (theme, dark) in [("dark", true), ("light", false)] {
            let mut shared = shared_for(&dir.join("history"), fonts.clone());
            shared.dark = dark;
            shared.config = Config::default();
            shared.ocr = stinkshot_platform::ocr::WindowsOcr::new()
                .ok()
                .map(|o| Arc::new(o) as _);
            let mut hw = crate::history_ui::HistoryWindow::new(&shared);
            shot(
                format!("shots-history-{theme}"),
                (1280.0, 800.0),
                vec![Step::Wait(2.0)],
                &mut shared,
                &mut hw,
            );
            let mut hw = crate::history_ui::HistoryWindow::new(&shared);
            shot(
                format!("shots-history-search-{theme}"),
                (1280.0, 800.0),
                vec![
                    Step::Wait(1.5),
                    Step::Key(Key::F, Modifiers::COMMAND),
                    Step::Wait(0.3),
                    Step::Type("shader", 0.05),
                    Step::Wait(2.0),
                ],
                &mut shared,
                &mut hw,
            );

            let mut shared = shared_for(&empty_history, fonts.clone());
            shared.dark = dark;
            shared.config = Config::default();
            let mut sw = crate::settings_ui::SettingsWindow::new(&shared);
            use crate::settings_ui::Section as S;
            for (page, section) in [
                ("general", S::General),
                ("privacy", S::Privacy),
                ("recording", S::Recording),
                ("about", S::About),
            ] {
                sw.show_section(section);
                shot(
                    format!("shots-settings-{page}-{theme}"),
                    (1040.0, 800.0),
                    vec![Step::Wait(1.5)],
                    &mut shared,
                    &mut sw,
                );
            }

            // The editor over the demo desktop: an arrow, numbered steps and a text label.
            let mut shared = shared_for(&empty_history, fonts.clone());
            shared.dark = dark;
            let mut o = overlay(&desktop, &windows, (882, 288), &shared);
            let steps = [
                Step::Wait(0.2),
                Step::Drag(1818.0, 842.0, 0.2),
                Step::Wait(0.3),
                key(Key::A),
                Step::Move(1610.0, 500.0, 0.2),
                Step::Drag(1736.0, 628.0, 0.3),
                Step::Wait(0.2),
                key(Key::N),
                Step::Move(1085.0, 375.0, 0.2),
                Step::Down,
                Step::Up,
                Step::Move(1315.0, 375.0, 0.2),
                Step::Down,
                Step::Up,
                Step::Move(1545.0, 375.0, 0.2),
                Step::Down,
                Step::Up,
                Step::Wait(0.2),
                key(Key::T),
                Step::Move(1300.0, 478.0, 0.2),
                Step::Down,
                Step::Up,
                Step::Wait(0.2),
                Step::Type("Fastest week yet", 0.03),
                Step::Wait(0.4),
                // First Escape ends the text edit, the second deselects.
                key(Key::Escape),
                Step::Wait(0.2),
                key(Key::Escape),
                Step::Move(1300.0, 1150.0, 0.3),
                Step::Wait(1.0),
                Step::Snap(Box::leak(format!("shots-editor-{theme}").into_boxed_str())),
            ];
            run_clip(
                &mut rig,
                &mut shared,
                &dir.join("shots-editor.mkv"),
                (882.0, 288.0),
                &steps,
                |ctx, sh| {
                    o.ensure_base_texture(ctx);
                    o.frame(ctx, sh);
                },
            );
        }
        crate::update::TEST_STORE_BUILD.with(|c| c.set(None));
    }
}
