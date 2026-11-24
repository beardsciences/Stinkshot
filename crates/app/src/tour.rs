//! First-run tour: four short, skippable steps (hotkeys, tray, privacy, where captures go).
//! The shell opens it once, on a plain launch while `general.tour_seen` is false; closing it
//! any way (Done, Skip, Esc, the close box) sets the flag. Settings > About > Show Tour opens it
//! again.

use std::time::Duration;

use egui::{CornerRadius, RichText, UiBuilder, pos2, vec2};
use stinkshot_core::config::Config;
use stinkshot_platform::Hwnd;

use crate::actions::Action;
use crate::l10n::{lookup, tr};
use crate::state::{Command, Shared, Surface, SurfaceStatus};
use crate::theme::{self, Icon, widgets};
use crate::toast::ui_common as uc;

/// The window title stays English: the shell finds an open tour by it (like History and
/// Settings).
const WINDOW_TITLE: &str = "Welcome to Stinkshot";

/// Which global hotkeys the first step lists (the rest are in Help).
const STEP_ACTIONS: [Action; 4] = [
    Action::Region,
    Action::Window,
    Action::Fullscreen,
    Action::RecordMp4,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Hotkeys,
    Tray,
    Privacy,
    Files,
}

const STEPS: [Step; 4] = [Step::Hotkeys, Step::Tray, Step::Privacy, Step::Files];

impl Step {
    fn icon(self) -> Icon {
        match self {
            Step::Hotkeys => Icon::Rect,
            Step::Tray => Icon::Clicks,
            Step::Privacy => Icon::Shield,
            Step::Files => Icon::Folder,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Step::Hotkeys => tr!("shell.tour.hotkeys.title"),
            Step::Tray => tr!("shell.tour.tray.title"),
            Step::Privacy => tr!("shell.tour.privacy.title"),
            Step::Files => tr!("shell.tour.files.title"),
        }
    }
}

/// The step body for this config.
fn body(step: Step, cfg: &Config) -> String {
    match step {
        Step::Hotkeys => {
            let key = crate::hotkeys::display(&cfg.hotkeys.region);
            if key.is_empty() {
                tr!("shell.tour.hotkeys.body_unset").into()
            } else {
                tr!("shell.tour.hotkeys.body", key = key)
            }
        }
        Step::Tray => tr!("shell.tour.tray.body").into(),
        Step::Privacy => tr!("shell.tour.privacy.body").into(),
        Step::Files => tr!("shell.tour.files.body").into(),
    }
}

/// Extra (key, label) rows under the body: the main hotkeys, or the captures folder.
fn detail_rows(step: Step, cfg: &Config) -> Vec<(String, String)> {
    match step {
        Step::Hotkeys => STEP_ACTIONS
            .iter()
            .map(|a| {
                let k = crate::hotkeys::display(a.hotkey(&cfg.hotkeys));
                let k = if k.is_empty() {
                    tr!("shell.hotkey.not_set").to_string()
                } else {
                    k
                };
                (k, a.label().to_string())
            })
            .collect(),
        Step::Files => vec![(cfg.history_folder().display().to_string(), String::new())],
        Step::Tray => [
            ("settings.help.tray.click", "settings.help.tray.click.what"),
            ("settings.help.tray.dbl", "settings.help.tray.dbl.what"),
            ("settings.help.tray.right", "settings.help.tray.right.what"),
        ]
        .iter()
        .map(|(k, v)| (lookup(k).to_string(), lookup(v).to_string()))
        .collect(),
        Step::Privacy => [
            (
                "shell.tour.privacy.captures",
                "shell.tour.privacy.captures.what",
            ),
            ("shell.tour.privacy.ocr", "shell.tour.privacy.ocr.what"),
            (
                "shell.tour.privacy.accounts",
                "shell.tour.privacy.accounts.what",
            ),
            (
                "shell.tour.privacy.updates",
                "shell.tour.privacy.updates.what",
            ),
        ]
        .iter()
        .map(|(k, v)| (lookup(k).to_string(), lookup(v).to_string()))
        .collect(),
    }
}

/// The tour logo's entrance at `t` seconds: (scale, squash). Pops in with an overshoot over
/// 0.45 s, then one squash that springs back.
fn logo_entrance(t: f32) -> (f32, f32) {
    let ease_out_back = |x: f32| {
        let (c, x) = (1.70158, x.clamp(0.0, 1.0) - 1.0);
        1.0 + (c + 1.0) * x * x * x + c * x * x
    };
    let scale = ease_out_back(t / 0.45);
    let p = ((t - 0.45) / 0.9).clamp(0.0, 1.0);
    let k = if t < 0.45 {
        0.0
    } else if p < 0.12 {
        p / 0.12
    } else {
        let q = (p - 0.12) / 0.88;
        let e = if q >= 1.0 {
            1.0
        } else {
            2f32.powf(-10.0 * q) * ((q * 10.0 - 0.75) * std::f32::consts::TAU / 3.0).sin() + 1.0
        };
        1.0 - e
    };
    (scale, k)
}

pub struct TourWindow {
    step: usize,
    done: bool,
    /// First frame shown: the logo's bounce-in on the first step.
    shown: Option<std::time::Instant>,
}

impl TourWindow {
    pub fn new() -> Self {
        Self {
            step: 0,
            done: false,
            shown: None,
        }
    }

    /// Opened on step `step` (0-based), for offscreen renders.
    #[cfg(test)]
    pub fn at_step(step: usize) -> Self {
        Self {
            step: step.min(STEPS.len() - 1),
            done: false,
            shown: None,
        }
    }

    /// The window title (the shell focuses an open tour by it).
    pub const TITLE: &'static str = WINDOW_TITLE;
}

impl Surface for TourWindow {
    fn title(&self) -> String {
        Self::TITLE.into()
    }

    fn viewport_builder(&self, _shared: &Shared) -> egui::ViewportBuilder {
        egui::ViewportBuilder::default()
            .with_title(self.title())
            .with_inner_size(vec2(560.0, 400.0))
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false)
    }

    fn on_created(&mut self, hwnd: Hwnd, shared: &Shared) {
        stinkshot_platform::window_util::apply_mica(hwnd, shared.dark);
        stinkshot_platform::window_util::force_foreground(hwnd);
    }

    fn show(&mut self, ctx: &egui::Context, shared: &mut Shared) -> SurfaceStatus {
        let p = uc::pal(shared);
        let accent = uc::accent(shared);
        let step = STEPS[self.step.min(STEPS.len() - 1)];
        let last = self.step + 1 >= STEPS.len();
        let mut close = ctx.input(|i| i.viewport().close_requested())
            || ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let (mut next, mut back, mut help) = (false, false, false);
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::Enter)) {
            next = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            back = true;
        }

        // The bounce starts once the logo is ready (a cold cache renders it first).
        let logo_ready = self.step == 0 && crate::logo_anim::orb_at(ctx, 0.0).is_some();
        let shown = if logo_ready {
            *self.shown.get_or_insert_with(std::time::Instant::now)
        } else {
            std::time::Instant::now()
        };
        uc::root_ui(ctx, p.bg, |ui, rect| {
            let area = rect.shrink2(vec2(32.0, 28.0));
            // Step dots, top right.
            for i in 0..STEPS.len() {
                let c = pos2(
                    area.right() - (STEPS.len() - 1 - i) as f32 * 14.0 - 4.0,
                    area.top() + 10.0,
                );
                let col = if i == self.step {
                    accent
                } else {
                    theme::subtle_fill(shared.dark, 3)
                };
                ui.painter().circle_filled(c, 4.0, col);
            }
            // First step: the logo bounces in where the step icon goes, then turns slowly.
            let t = shown.elapsed().as_secs_f32();
            let orb = (self.step == 0)
                .then(|| {
                    crate::logo_anim::orb_at(
                        ui.ctx(),
                        6.0 / crate::logo_anim::FRAMES as f64 + t as f64 / 12.0,
                    )
                })
                .flatten();
            if let Some(tex) = orb {
                let (scale, k) = if crate::logo_anim::animations_on() {
                    logo_entrance(t)
                } else {
                    (1.0, 0.0)
                };
                let side = 64.0 * scale;
                let c = area.min + vec2(18.0, 16.0 + 3.0 * k);
                ui.painter().image(
                    tex,
                    egui::Rect::from_center_size(
                        c,
                        vec2(side * (1.0 + 0.16 * k), side * (1.0 - 0.14 * k)),
                    ),
                    egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                ui.ctx().request_repaint_after(Duration::from_millis(16));
            } else {
                widgets::paint_icon(
                    ui.painter(),
                    egui::Rect::from_min_size(area.min, vec2(36.0, 36.0)),
                    step.icon(),
                    accent,
                );
            }
            let content = egui::Rect::from_min_max(area.min + vec2(0.0, 52.0), area.max);
            ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(
                    RichText::new(step.title())
                        .font(theme::semibold(22.0))
                        .color(p.text),
                );
                ui.label(
                    RichText::new(body(step, &shared.config))
                        .size(14.0)
                        .color(p.text_secondary),
                );
                let rows = detail_rows(step, &shared.config);
                if !rows.is_empty() {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(p.card)
                        .stroke(egui::Stroke::new(1.0, p.stroke))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(16, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for (k, v) in &rows {
                                ui.horizontal(|ui| {
                                    if v.is_empty() {
                                        ui.label(RichText::new(k).size(13.0).color(p.text));
                                    } else {
                                        // A real label (screen readers read the key), at
                                        // least 150 wide so the names line up.
                                        ui.allocate_ui_with_layout(
                                            vec2(150.0, 20.0),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_min_width(150.0);
                                                ui.label(
                                                    RichText::new(k)
                                                        .font(theme::semibold(13.0))
                                                        .color(p.text),
                                                );
                                            },
                                        );
                                        ui.label(
                                            RichText::new(v).size(13.0).color(p.text_secondary),
                                        );
                                    }
                                });
                            }
                        });
                }
            });

            // Buttons, bottom row.
            let bar = egui::Rect::from_min_max(pos2(area.left(), area.bottom() - 34.0), area.max);
            ui.scope_builder(UiBuilder::new().max_rect(bar), |ui| {
                ui.horizontal_centered(|ui| {
                    if !last && widgets::button(ui, tr!("shell.tour.skip")).clicked() {
                        close = true;
                    }
                    if last && widgets::button(ui, tr!("shell.tour.open_help")).clicked() {
                        help = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let primary = if last {
                            tr!("shell.tour.done")
                        } else {
                            tr!("shell.tour.next")
                        };
                        if widgets::accent_button(ui, primary).clicked() {
                            next = true;
                        }
                        if self.step > 0 && widgets::button(ui, tr!("shell.tour.back")).clicked() {
                            back = true;
                        }
                    });
                });
            });
        });

        if back && self.step > 0 {
            self.step -= 1;
        } else if next {
            if last {
                close = true;
            } else {
                self.step += 1;
            }
        }
        if help {
            shared
                .outbox
                .push(Command::OpenSettingsAt(crate::settings_ui::Section::Help));
            close = true;
        }
        if close && !self.done {
            self.done = true;
            if !shared.config.general.tour_seen {
                shared.config.general.tour_seen = true;
                shared.config_dirty = true;
            }
        }
        // Keep the dots and hover states fresh without spinning.
        ctx.request_repaint_after(Duration::from_secs(1));
        if close {
            SurfaceStatus::Close
        } else {
            SurfaceStatus::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_steps_with_the_privacy_promise() {
        assert_eq!(STEPS.len(), 4);
        assert!(
            STEPS
                .iter()
                .any(|s| s.title() == "Your Captures Never Leave Your PC")
        );
    }

    #[test]
    fn hotkey_step_names_the_region_key_or_says_how_to_set_one() {
        let mut cfg = Config::default();
        cfg.hotkeys.region = "PrintScreen".into();
        assert!(body(Step::Hotkeys, &cfg).contains("Press PrtSc"));
        cfg.hotkeys.region.clear();
        assert_eq!(
            body(Step::Hotkeys, &cfg),
            tr!("shell.tour.hotkeys.body_unset")
        );
        let rows = detail_rows(Step::Hotkeys, &cfg);
        assert_eq!(rows.len(), STEP_ACTIONS.len());
        assert_eq!(
            rows[0],
            (
                tr!("shell.hotkey.not_set").into(),
                Action::Region.label().into()
            )
        );
    }

    #[test]
    fn files_step_shows_the_captures_folder() {
        let cfg = Config::default();
        let rows = detail_rows(Step::Files, &cfg);
        assert_eq!(rows[0].0, cfg.history_folder().display().to_string());
    }

    #[test]
    fn bodies_are_short() {
        let cfg = Config::default();
        for s in STEPS {
            assert!(body(s, &cfg).len() <= 70, "{s:?}");
        }
    }

    #[test]
    fn tour_is_unseen_by_default() {
        assert!(!Config::default().general.tour_seen);
    }
}
