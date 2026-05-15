use std::env;
use std::time::Duration;

use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke, TextStyle, Vec2};

use crate::audio_engine::{DemoEntry, DemoOverview, GuiAudioEngine, RenderState};
use crate::ffi::AudioRenderStats;
use crate::playback::{PlaybackController, PlaybackState};

const DEFAULT_STATUS_LINE: &str = "READY. SELECT A DEMO AND PRESS ENTER TO RENDER.";
const BASE_BG: Color32 = Color32::from_rgb(10, 12, 10);
const PANEL_BG: Color32 = Color32::from_rgb(16, 20, 16);
const PANEL_ALT_BG: Color32 = Color32::from_rgb(20, 26, 20);
const PANEL_DIM_BG: Color32 = Color32::from_rgb(12, 16, 12);
const BORDER: Color32 = Color32::from_rgb(88, 106, 88);
const BORDER_DIM: Color32 = Color32::from_rgb(58, 72, 58);
const TEXT: Color32 = Color32::from_rgb(214, 226, 214);
const TEXT_DIM: Color32 = Color32::from_rgb(136, 150, 136);
const ACCENT: Color32 = Color32::from_rgb(130, 214, 144);
const ACCENT_SOFT: Color32 = Color32::from_rgb(72, 108, 76);
const WARNING: Color32 = Color32::from_rgb(234, 122, 106);
const WAVEFORM: Color32 = Color32::from_rgb(194, 222, 194);
const GRID: Color32 = Color32::from_rgb(34, 44, 34);

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    DemoList,
    Overview,
}

#[derive(Default)]
struct BootOptions {
    demo_key: Option<String>,
    auto_render: bool,
    focus: Option<FocusArea>,
}

struct RuntimeState {
    selected_demo: usize,
    loaded_metadata: Option<DemoOverview>,
    rendered_audio: Option<RenderState>,
    focus: FocusArea,
    last_error: Option<String>,
}

pub struct MemDeckGuiApp {
    audio_engine: GuiAudioEngine,
    playback: PlaybackController,
    demos: Vec<DemoEntry>,
    status_line: String,
    runtime: RuntimeState,
    boot_options: BootOptions,
    boot_applied: bool,
}

impl Default for MemDeckGuiApp {
    fn default() -> Self {
        let audio_engine = GuiAudioEngine::new();
        let demos = audio_engine.demo_catalog();
        let boot_options = BootOptions::from_env();
        let selected_demo = boot_options
            .demo_key
            .as_deref()
            .and_then(|demo_key| demos.iter().position(|demo| demo.key == demo_key))
            .unwrap_or(0);
        let loaded_metadata = demos.get(selected_demo).and_then(|demo| demo.overview.clone());
        let mut status_line = DEFAULT_STATUS_LINE.to_string();
        let mut last_error = None;
        if let Some(error) = demos.get(selected_demo).and_then(|demo| demo.error.clone()) {
            status_line = format!("DEMO ERROR • {error}");
            last_error = Some(error);
        }

        Self {
            audio_engine,
            playback: PlaybackController::default(),
            demos,
            status_line,
            runtime: RuntimeState {
                selected_demo,
                loaded_metadata,
                rendered_audio: None,
                focus: boot_options.focus.unwrap_or(FocusArea::DemoList),
                last_error,
            },
            boot_options,
            boot_applied: false,
        }
    }
}

impl MemDeckGuiApp {
    pub fn configure_visuals(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = BASE_BG;
        visuals.panel_fill = BASE_BG;
        visuals.faint_bg_color = PANEL_DIM_BG;
        visuals.extreme_bg_color = BASE_BG;
        visuals.override_text_color = Some(TEXT);
        visuals.selection.bg_fill = ACCENT_SOFT;
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.noninteractive.bg_fill = PANEL_BG;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_DIM);
        visuals.widgets.inactive.bg_fill = PANEL_DIM_BG;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_DIM);
        visuals.widgets.hovered.bg_fill = PANEL_ALT_BG;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.active.bg_fill = PANEL_ALT_BG;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(10.0);
        style.visuals.window_stroke = Stroke::new(1.0, BORDER);
        style.text_styles = [
            (TextStyle::Heading, FontId::monospace(21.0)),
            (TextStyle::Name("Title".into()), FontId::monospace(18.0)),
            (TextStyle::Body, FontId::monospace(14.0)),
            (TextStyle::Button, FontId::monospace(14.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
            (TextStyle::Small, FontId::monospace(12.0)),
        ]
        .into();
        ctx.set_style(style);
    }

    fn selected_demo(&self) -> &DemoEntry {
        &self.demos[self.runtime.selected_demo]
    }

    fn selected_overview(&self) -> Option<&DemoOverview> {
        self.runtime.loaded_metadata.as_ref()
    }

    fn current_render(&self) -> Option<&RenderState> {
        self.runtime
            .rendered_audio
            .as_ref()
            .filter(|render| render.demo_key == self.selected_demo().key)
    }

    fn current_stats(&self) -> Option<AudioRenderStats> {
        self.current_render().and_then(|render| render.stats)
    }

    fn apply_boot_options(&mut self) {
        if self.boot_applied {
            return;
        }

        if self.boot_options.auto_render {
            let _ = self.render_selected_demo();
        }

        self.boot_applied = true;
    }

    fn select_demo(&mut self, index: usize) {
        // Guard: out of bounds or already selected.
        if index >= self.demos.len() || index == self.runtime.selected_demo {
            return;
        }

        self.stop_playback(false);
        self.runtime.selected_demo = index;

        // Snapshot the fields we need before any further &mut borrow.
        let (overview, error, key) = {
            let demo = &self.demos[index];
            (demo.overview.clone(), demo.error.clone(), demo.key.clone())
        };

        self.runtime.loaded_metadata = overview;
        if let Some(error) = error {
            self.runtime.last_error = Some(error.clone());
            self.status_line = format!("DEMO ERROR • {error}");
        } else {
            self.runtime.last_error = None;
            self.status_line = format!("SELECTED {}.", key.to_uppercase());
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.demos.is_empty() {
            return;
        }

        let next = (self.runtime.selected_demo as isize + delta)
            .clamp(0, self.demos.len().saturating_sub(1) as isize) as usize;
        self.select_demo(next);
    }

    fn render_selected_demo(&mut self) -> Result<(), String> {
        self.stop_playback(false);
        let demo = self.selected_demo().clone();

        match self.audio_engine.render_demo(&demo.key, &demo.path) {
            Ok(render) => {
                let sample_count = render.samples.len();
                let stats = render.stats;
                self.runtime.rendered_audio = Some(render);
                self.runtime.last_error = None;
                self.status_line = if let Some(stats) = stats {
                    format!(
                        "RENDER OK • {} • {} SAMPLES • CHECKSUM {:016X}",
                        demo.key.to_uppercase(),
                        stats.sample_count,
                        stats.checksum
                    )
                } else {
                    format!(
                        "RENDER OK • {} • {} SAMPLES",
                        demo.key.to_uppercase(),
                        sample_count
                    )
                };
                Ok(())
            }
            Err(error) => {
                if self
                    .runtime
                    .rendered_audio
                    .as_ref()
                    .is_some_and(|render| render.demo_key == demo.key)
                {
                    self.runtime.rendered_audio = None;
                }
                let message = format!("RENDER ERROR • {} • {error}", demo.key.to_uppercase());
                self.status_line = message.clone();
                self.runtime.last_error = Some(message.clone());
                Err(message)
            }
        }
    }

    fn ensure_render_for_selected(&mut self) -> Result<(), String> {
        if self.current_render().is_some() {
            return Ok(());
        }

        self.render_selected_demo()
    }

    fn toggle_playback(&mut self) {
        if self.playback.is_playing() {
            self.stop_playback(true);
            return;
        }

        if let Err(error) = self.ensure_render_for_selected() {
            self.status_line = error;
            self.runtime.last_error = Some(self.status_line.clone());
            return;
        }

        let demo_name = self.selected_demo().key.clone();
        let samples = self
            .current_render()
            .map(|render| render.samples.clone())
            .unwrap_or_default();

        if samples.is_empty() {
            self.status_line = format!("PLAYBACK ERROR • {} HAS NO PCM.", demo_name.to_uppercase());
            self.runtime.last_error = Some(self.status_line.clone());
            return;
        }

        match self.playback.start_pcm(&samples) {
            Ok(()) => {
                self.status_line = format!("PLAYING • {}", demo_name.to_uppercase());
                self.runtime.last_error = None;
            }
            Err(error) => {
                self.status_line = format!("PLAYBACK ERROR • {error}");
                self.runtime.last_error = Some(self.status_line.clone());
            }
        }
    }

    fn stop_playback(&mut self, update_status: bool) {
        match self.playback.stop() {
            Ok(true) if update_status => {
                self.status_line = "STOPPED PLAYBACK.".to_string();
                self.runtime.last_error = None;
            }
            Ok(_) => {}
            Err(error) if update_status => {
                self.status_line = format!("STOP ERROR • {error}");
                self.runtime.last_error = Some(self.status_line.clone());
            }
            Err(_) => {}
        }
    }

    fn poll_playback(&mut self) {
        if let Some(result) = self.playback.poll() {
            self.status_line = match result {
                Ok(()) => "PLAYBACK FINISHED.".to_string(),
                Err(error) => format!("PLAYBACK ERROR • {error}"),
            };
            self.runtime.last_error = match self.playback.state() {
                PlaybackState::Error(message) => Some(message.clone()),
                _ => None,
            };
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Tab) {
                self.runtime.focus = match self.runtime.focus {
                    FocusArea::DemoList => FocusArea::Overview,
                    FocusArea::Overview => FocusArea::DemoList,
                };
            }

            if input.key_pressed(egui::Key::ArrowUp) {
                self.move_selection(-1);
            }

            if input.key_pressed(egui::Key::ArrowDown) {
                self.move_selection(1);
            }

            if input.key_pressed(egui::Key::Enter) {
                let _ = self.render_selected_demo();
            }

            if input.key_pressed(egui::Key::Space) {
                self.toggle_playback();
            }

            if input.key_pressed(egui::Key::Escape) {
                self.stop_playback(true);
            }
        });
    }

    fn draw_demo_list(&mut self, ui: &mut egui::Ui) {
        let focus = self.runtime.focus == FocusArea::DemoList;
        Self::retro_panel(ui, "DEMOS", focus, |ui| {
            ui.add_space(2.0);
            for index in 0..self.demos.len() {
                let selected = self.runtime.selected_demo == index;
                let available = self.demos[index].overview.is_some();
                let prefix = if selected { ">" } else { " " };
                let label = format!("{prefix} {}", self.demos[index].key);
                let text_color = if !available {
                    WARNING
                } else if selected {
                    BASE_BG
                } else {
                    TEXT
                };
                let fill = if selected { ACCENT } else { PANEL_BG };
                let stroke = if selected {
                    Stroke::new(1.0, ACCENT)
                } else {
                    Stroke::new(1.0, BORDER_DIM)
                };

                let button = egui::Button::new(
                    RichText::new(label)
                        .monospace()
                        .size(14.0)
                        .color(text_color),
                )
                .fill(fill)
                .stroke(stroke)
                .min_size(Vec2::new(ui.available_width(), 22.0));

                if ui.add(button).clicked() {
                    self.runtime.focus = FocusArea::DemoList;
                    self.select_demo(index);
                }
            }

            if let Some(error) = &self.selected_demo().error {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(error)
                        .monospace()
                        .color(WARNING)
                        .size(12.0),
                );
            }
        });
    }

    fn draw_stats_panel(&self, ui: &mut egui::Ui) {
        let focus = self.runtime.focus == FocusArea::Overview;
        let demo = self.selected_demo();
        let overview = self.selected_overview();
        let stats = self.current_stats();
        let is_playing = matches!(self.playback.state(), PlaybackState::Playing);

        Self::retro_panel(ui, "RENDER STATS", focus, |ui| {
            ui.label(
                RichText::new(overview.map_or(demo.key.as_str(), |view| view.title.as_str()))
                    .monospace()
                    .size(16.0)
                    .strong(),
            );
            ui.label(
                RichText::new(if is_playing { "STATE  PLAYING" } else { "STATE  IDLE" })
                    .monospace()
                    .color(if is_playing { ACCENT } else { TEXT_DIM }),
            );
            ui.add_space(4.0);

            egui::Grid::new("runtime_stats_grid")
                .num_columns(2)
                .spacing(egui::vec2(14.0, 6.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("demo").monospace().color(TEXT_DIM));
                    ui.label(RichText::new(&demo.key).monospace().color(TEXT));
                    ui.end_row();

                    ui.label(RichText::new("bpm").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            overview
                                .map(|value| value.bpm.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("swing").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            overview
                                .map(|value| format!("{}%", value.swing_pct))
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("duration").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| format!("{:.2} ms", value.duration_ms))
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("samples").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| value.sample_count.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("clipping").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| value.clipping_count.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace()
                        .color(if stats.is_some_and(|value| value.clipping_count > 0) {
                            WARNING
                        } else {
                            TEXT
                        }),
                    );
                    ui.end_row();

                    ui.label(RichText::new("peak").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| value.peak.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("min/max").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| format!("{} / {}", value.min_sample, value.max_sample))
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("render ms").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| format!("{:.2} ms", value.render_time_ms))
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("checksum").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(
                            stats
                                .map(|value| format!("{:016X}", value.checksum))
                                .unwrap_or_else(|| "--".to_string()),
                        )
                        .monospace(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("render").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new(if stats.is_some() { "success" } else { "pending" })
                            .monospace()
                            .color(if stats.is_some() { ACCENT } else { TEXT_DIM }),
                    );
                    ui.end_row();
                });

            if let Some(overview) = overview {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "PATTERNS {} • STEPS {} • TRACKS {}",
                        overview.arrangement.len(),
                        overview.total_steps,
                        overview.tracks.len()
                    ))
                    .monospace()
                    .color(TEXT_DIM),
                );
                if overview.hidden_track_count > 0 {
                    ui.label(
                        RichText::new(format!(
                            "RENDERER SHOWS FIRST {} TRACKS • {} HIDDEN",
                            overview.tracks.len(),
                            overview.hidden_track_count
                        ))
                        .monospace()
                        .color(TEXT_DIM)
                        .size(12.0),
                    );
                }
            }
        });
    }

    fn draw_waveform_and_pattern(&self, ui: &mut egui::Ui) {
        let focus = self.runtime.focus == FocusArea::Overview;
        let render = self.current_render();
        let overview = self.selected_overview();

        Self::retro_panel(ui, "WAVEFORM / PATTERN OVERVIEW", focus, |ui| {
            ui.label(
                RichText::new("PCM MINIMAP")
                    .monospace()
                    .size(13.0)
                    .color(TEXT_DIM),
            );
            Self::draw_waveform_minimap(ui, render.map(|value| value.samples.as_slice()));
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                RichText::new("ARRANGEMENT + STEP ACTIVITY")
                    .monospace()
                    .size(13.0)
                    .color(TEXT_DIM),
            );
            Self::draw_pattern_visualization(ui, overview);
        });
    }

    fn draw_waveform_minimap(ui: &mut egui::Ui, samples: Option<&[u8]>) {
        let desired_size = egui::vec2(ui.available_width(), 150.0);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
        let rect = response.rect.shrink(4.0);

        painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));

        let mid_y = rect.center().y;
        painter.line_segment(
            [egui::pos2(rect.left(), mid_y), egui::pos2(rect.right(), mid_y)],
            Stroke::new(1.0, GRID),
        );

        for fraction in [0.25_f32, 0.5, 0.75] {
            let x = egui::lerp(rect.left()..=rect.right(), fraction);
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(1.0, GRID),
            );
        }

        let Some(samples) = samples.filter(|samples| !samples.is_empty()) else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "RENDER TO PREVIEW PCM",
                FontId::monospace(14.0),
                TEXT_DIM,
            );
            return;
        };

        let columns = rect.width().max(64.0) as usize;
        let stride = (samples.len() / columns).max(1);
        let mut upper = Vec::new();
        let mut lower = Vec::new();
        let mut clip_markers = Vec::new();
        let usable_width = (rect.width() - 8.0).max(1.0);

        for (index, chunk) in samples.chunks(stride).take(columns).enumerate() {
            let x = rect.left() + 4.0 + usable_width * (index as f32 / columns as f32);
            let mut min_value = 1.0_f32;
            let mut max_value = -1.0_f32;
            let mut clipped = false;

            for &sample in chunk {
                let normalized = (sample as f32 - 128.0) / 128.0;
                min_value = min_value.min(normalized);
                max_value = max_value.max(normalized);
                clipped |= sample == 0 || sample == 255;
            }

            upper.push(egui::pos2(
                x,
                egui::remap(max_value, -1.0..=1.0, rect.bottom()..=rect.top()),
            ));
            lower.push(egui::pos2(
                x,
                egui::remap(min_value, -1.0..=1.0, rect.bottom()..=rect.top()),
            ));
            if clipped {
                clip_markers.push(x);
            }
        }

        painter.add(egui::Shape::line(upper, Stroke::new(1.5, WAVEFORM)));
        painter.add(egui::Shape::line(lower, Stroke::new(1.5, WAVEFORM)));

        for x in clip_markers {
            painter.line_segment(
                [egui::pos2(x, rect.top() + 4.0), egui::pos2(x, rect.top() + 18.0)],
                Stroke::new(1.5, WARNING),
            );
        }
    }

    fn draw_pattern_visualization(ui: &mut egui::Ui, overview: Option<&DemoOverview>) {
        let Some(overview) = overview else {
            ui.label(
                RichText::new("PATTERN DATA NOT AVAILABLE FOR THIS DEMO.")
                    .monospace()
                    .color(WARNING),
            );
            return;
        };

        if overview.arrangement.is_empty() || overview.tracks.is_empty() {
            ui.label(
                RichText::new("NO PATTERN ARRANGEMENT FOUND.")
                    .monospace()
                    .color(TEXT_DIM),
            );
            return;
        }

        let header_height = 26.0;
        let row_height = 24.0;
        let desired_height = header_height + row_height * overview.tracks.len() as f32 + 10.0;
        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width(), desired_height.max(120.0)),
            Sense::hover(),
        );
        let rect = response.rect.shrink(4.0);
        let label_width = rect.width().min(220.0).max(150.0);
        let timeline_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + label_width, rect.top()),
            egui::pos2(rect.right(), rect.bottom()),
        );
        let total_steps = overview.total_steps.max(1) as f32;

        painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));
        painter.line_segment(
            [
                egui::pos2(timeline_rect.left(), rect.top()),
                egui::pos2(timeline_rect.left(), rect.bottom()),
            ],
            Stroke::new(1.0, BORDER_DIM),
        );

        for block in &overview.arrangement {
            let x0 = egui::lerp(timeline_rect.left()..=timeline_rect.right(), block.start_step as f32 / total_steps);
            let x1 = egui::lerp(
                timeline_rect.left()..=timeline_rect.right(),
                (block.start_step + block.length) as f32 / total_steps,
            );
            let block_rect = egui::Rect::from_min_max(
                egui::pos2(x0, rect.top()),
                egui::pos2(x1.max(x0 + 1.0), rect.top() + header_height),
            );
            painter.rect_filled(block_rect, 0.0, PANEL_ALT_BG);
            painter.rect_stroke(block_rect, 0.0, Stroke::new(1.0, BORDER_DIM));
            painter.text(
                block_rect.center(),
                Align2::CENTER_CENTER,
                &block.label,
                FontId::monospace(12.0),
                ACCENT,
            );
        }

        for (track_index, track) in overview.tracks.iter().enumerate() {
            let row_top = rect.top() + header_height + track_index as f32 * row_height;
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), row_top),
                egui::pos2(rect.right(), row_top + row_height),
            );
            let row_fill = if track_index % 2 == 0 { PANEL_DIM_BG } else { PANEL_BG };
            painter.rect_filled(row_rect, 0.0, row_fill);
            painter.line_segment(
                [
                    egui::pos2(rect.left(), row_rect.bottom()),
                    egui::pos2(rect.right(), row_rect.bottom()),
                ],
                Stroke::new(1.0, GRID),
            );
            painter.text(
                egui::pos2(rect.left() + 6.0, row_rect.center().y),
                Align2::LEFT_CENTER,
                format!("{} / {}", track.name, track.instrument),
                FontId::monospace(12.0),
                TEXT,
            );

            for block in &overview.arrangement {
                let x = egui::lerp(timeline_rect.left()..=timeline_rect.right(), block.start_step as f32 / total_steps);
                painter.line_segment(
                    [egui::pos2(x, row_rect.top()), egui::pos2(x, row_rect.bottom())],
                    Stroke::new(1.0, BORDER_DIM),
                );
            }

            for (step_index, active) in track.activity.iter().enumerate() {
                if !active || step_index >= overview.total_steps {
                    continue;
                }

                let x0 = egui::lerp(timeline_rect.left()..=timeline_rect.right(), step_index as f32 / total_steps);
                let x1 = egui::lerp(
                    timeline_rect.left()..=timeline_rect.right(),
                    (step_index + 1) as f32 / total_steps,
                );
                let cell_rect = egui::Rect::from_min_max(
                    egui::pos2(x0 + 0.5, row_rect.top() + 4.0),
                    egui::pos2((x1 - 1.0).max(x0 + 1.5), row_rect.bottom() - 4.0),
                );
                painter.rect_filled(cell_rect, 0.0, ACCENT_SOFT);
                painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, ACCENT));
            }
        }
    }

    fn draw_footer(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("[TAB] FOCUS")
                    .monospace()
                    .size(13.0)
                    .color(TEXT_DIM),
            );
            ui.separator();
            ui.label(
                RichText::new("[UP/DOWN] SELECT")
                    .monospace()
                    .size(13.0)
                    .color(TEXT_DIM),
            );
            ui.separator();
            ui.label(
                RichText::new("[ENTER] RENDER")
                    .monospace()
                    .size(13.0)
                    .color(TEXT),
            );
            ui.separator();
            ui.label(
                RichText::new("[SPACE] PLAY / STOP")
                    .monospace()
                    .size(13.0)
                    .color(TEXT),
            );
            ui.separator();
            ui.label(
                RichText::new("[ESC] STOP")
                    .monospace()
                    .size(13.0)
                    .color(TEXT),
            );
        });
    }

    fn retro_panel<R>(
        ui: &mut egui::Ui,
        title: &str,
        focused: bool,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let frame = egui::Frame::group(ui.style())
            .fill(PANEL_BG)
            .inner_margin(egui::Margin::same(10.0))
            .stroke(Stroke::new(if focused { 2.0 } else { 1.0 }, if focused { ACCENT } else { BORDER }));

        frame
            .show(ui, |ui| {
                ui.label(
                    RichText::new(title)
                        .monospace()
                        .size(14.0)
                        .strong()
                        .color(if focused { ACCENT } else { TEXT }),
                );
                ui.add_space(6.0);
                add_contents(ui)
            })
            .inner
    }
}

impl Drop for MemDeckGuiApp {
    fn drop(&mut self) {
        let _ = self.playback.stop();
    }
}

impl eframe::App for MemDeckGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_playback();
        self.apply_boot_options();
        self.handle_keyboard(ctx);

        if self.playback.is_playing() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        egui::TopBottomPanel::top("runtime_header")
            .exact_height(52.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("MEMDECK SOUND MACHINE");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(match self.runtime.focus {
                                FocusArea::DemoList => "FOCUS  DEMOS",
                                FocusArea::Overview => "FOCUS  OVERVIEW",
                            })
                            .monospace()
                            .color(TEXT_DIM),
                        );
                    });
                });
                ui.separator();
            });

        egui::TopBottomPanel::bottom("runtime_footer")
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(&self.status_line)
                        .monospace()
                        .color(if self.runtime.last_error.is_some() {
                            WARNING
                        } else if self.playback.is_playing() {
                            ACCENT
                        } else {
                            TEXT
                        }),
                );
                ui.separator();
                self.draw_footer(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.columns(2, |columns| {
                    self.draw_demo_list(&mut columns[0]);
                    self.draw_stats_panel(&mut columns[1]);
                });
                ui.add_space(8.0);
                self.draw_waveform_and_pattern(ui);
            });
        });
    }
}

impl BootOptions {
    fn from_env() -> Self {
        Self {
            demo_key: env::var("MEMDECK_GUI_BOOT_DEMO").ok(),
            auto_render: env_flag("MEMDECK_GUI_BOOT_RENDER"),
            focus: match env::var("MEMDECK_GUI_BOOT_FOCUS").ok().as_deref() {
                Some("overview") => Some(FocusArea::Overview),
                Some("demos") => Some(FocusArea::DemoList),
                _ => None,
            },
        }
    }
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}
