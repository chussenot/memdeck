use std::env;
use std::time::Duration;

use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke, TextStyle, Vec2};

use crate::audio_engine::{
    AudioRenderStats, DemoEntry, DemoOverview, FxBusOverview, GuiAudioEngine, PatternBlock,
    RenderState, TrackOverview,
};
use crate::playback::{PlaybackController, PlaybackState};

const DEFAULT_STATUS_LINE: &str = "READY • SELECT A DEMO • ENTER RENDERS • SPACE PLAYS.";
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
const ACCENT_DIM: Color32 = Color32::from_rgb(52, 80, 56);
const WARNING: Color32 = Color32::from_rgb(234, 122, 106);
const WAVEFORM: Color32 = Color32::from_rgb(194, 222, 194);
const GRID: Color32 = Color32::from_rgb(34, 44, 34);

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    DemoBrowser,
    RenderStats,
    WaveformView,
    PatternOverview,
    InstrumentInspector,
    FxInspector,
}

impl FocusArea {
    const ALL: [FocusArea; 6] = [
        FocusArea::DemoBrowser,
        FocusArea::RenderStats,
        FocusArea::WaveformView,
        FocusArea::PatternOverview,
        FocusArea::InstrumentInspector,
        FocusArea::FxInspector,
    ];

    fn title(self) -> &'static str {
        match self {
            FocusArea::DemoBrowser => "DEMO BROWSER",
            FocusArea::RenderStats => "RENDER STATS",
            FocusArea::WaveformView => "WAVEFORM VIEW",
            FocusArea::PatternOverview => "PATTERN OVERVIEW",
            FocusArea::InstrumentInspector => "INSTRUMENT INSPECTOR",
            FocusArea::FxInspector => "FX INSPECTOR",
        }
    }

    fn boot_key(self) -> &'static str {
        match self {
            FocusArea::DemoBrowser => "demos",
            FocusArea::RenderStats => "stats",
            FocusArea::WaveformView => "waveform",
            FocusArea::PatternOverview => "pattern",
            FocusArea::InstrumentInspector => "instrument",
            FocusArea::FxInspector => "fx",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusTone {
    Normal,
    Active,
    Warning,
}

struct StatusMessage {
    text: String,
    tone: StatusTone,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            text: DEFAULT_STATUS_LINE.to_string(),
            tone: StatusTone::Normal,
        }
    }
}

#[derive(Default)]
struct BootOptions {
    demo_key: Option<String>,
    auto_render: bool,
    focus: Option<FocusArea>,
}

struct RuntimeState {
    selected_demo: usize,
    selected_track: usize,
    rendered_audio: Option<RenderState>,
    focus: FocusArea,
}

pub struct MemDeckGuiApp {
    audio_engine: GuiAudioEngine,
    playback: PlaybackController,
    demos: Vec<DemoEntry>,
    runtime: RuntimeState,
    status: StatusMessage,
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
        let mut status = StatusMessage::default();

        if let Some(error) = demos
            .get(selected_demo)
            .and_then(|demo| demo.error.as_ref())
        {
            status = StatusMessage {
                text: format!("DEMO ERROR • {error}"),
                tone: StatusTone::Warning,
            };
        }

        Self {
            audio_engine,
            playback: PlaybackController::default(),
            demos,
            runtime: RuntimeState {
                selected_demo,
                selected_track: 0,
                rendered_audio: None,
                focus: boot_options.focus.unwrap_or(FocusArea::DemoBrowser),
            },
            status,
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
        self.selected_demo().overview.as_ref()
    }

    fn selected_track(&self) -> Option<&TrackOverview> {
        let overview = self.selected_overview()?;
        overview.tracks.get(self.clamped_track_index(overview))
    }

    fn selected_fx_bus(&self) -> Option<&FxBusOverview> {
        let overview = self.selected_overview()?;
        let track = self.selected_track()?;
        overview.fx_buses.get(track.fx_bus)
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

    fn playback_progress(&self) -> Option<f32> {
        if self.playback.is_playing() {
            self.playback.progress()
        } else {
            None
        }
    }

    fn focus_index(&self) -> usize {
        FocusArea::ALL
            .iter()
            .position(|area| *area == self.runtime.focus)
            .unwrap_or(0)
    }

    fn focus_label(&self) -> &'static str {
        self.runtime.focus.title()
    }

    fn clamped_track_index(&self, overview: &DemoOverview) -> usize {
        self.runtime
            .selected_track
            .min(overview.tracks.len().saturating_sub(1))
    }

    fn sync_selected_track(&mut self) {
        if let Some(overview) = self.selected_overview() {
            self.runtime.selected_track = self.clamped_track_index(overview);
        } else {
            self.runtime.selected_track = 0;
        }
    }

    fn set_status(&mut self, tone: StatusTone, message: impl Into<String>) {
        self.status = StatusMessage {
            text: message.into(),
            tone,
        };
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
        if index >= self.demos.len() || index == self.runtime.selected_demo {
            return;
        }

        self.stop_playback(false);
        self.runtime.selected_demo = index;
        self.runtime.selected_track = 0;

        let (error, key) = {
            let demo = &self.demos[index];
            (demo.error.clone(), demo.key.clone())
        };

        if let Some(error) = error {
            self.set_status(StatusTone::Warning, format!("DEMO ERROR • {error}"));
        } else {
            self.set_status(
                StatusTone::Normal,
                format!(
                    "SELECTED • {} • ENTER RENDERS • SPACE PLAYS",
                    key.to_uppercase()
                ),
            );
        }
    }

    fn move_demo_selection(&mut self, delta: isize) {
        if self.demos.is_empty() {
            return;
        }

        let next = (self.runtime.selected_demo as isize + delta)
            .clamp(0, self.demos.len().saturating_sub(1) as isize) as usize;
        self.select_demo(next);
    }

    fn move_track_selection(&mut self, delta: isize) {
        let Some((track_count, next, label)) = self.selected_overview().map(|overview| {
            let track_count = overview.tracks.len();
            if track_count == 0 {
                return (0, 0, None);
            }

            let next = (self.clamped_track_index(overview) as isize + delta)
                .clamp(0, track_count.saturating_sub(1) as isize) as usize;
            let label = overview
                .tracks
                .get(next)
                .map(|track| (track.name.clone(), track.fx_bus));
            (track_count, next, label)
        }) else {
            return;
        };
        if track_count == 0 {
            return;
        }

        self.runtime.selected_track = next;
        if let Some((track_name, fx_bus)) = label {
            self.set_status(
                StatusTone::Normal,
                format!(
                    "TRACK {:02} • {} • BUS {}",
                    next + 1,
                    track_name.to_uppercase(),
                    fx_bus
                ),
            );
        }
    }

    fn cycle_focus(&mut self, reverse: bool) {
        let mut index = self.focus_index() as isize;
        index += if reverse { -1 } else { 1 };
        if index < 0 {
            index = FocusArea::ALL.len() as isize - 1;
        }
        if index as usize >= FocusArea::ALL.len() {
            index = 0;
        }
        self.runtime.focus = FocusArea::ALL[index as usize];
        self.set_status(
            StatusTone::Normal,
            format!("FOCUS • {}", self.runtime.focus.title()),
        );
    }

    fn focus_panel(&mut self, focus: FocusArea) {
        self.runtime.focus = focus;
        self.set_status(StatusTone::Normal, format!("FOCUS • {}", focus.title()));
    }

    fn render_selected_demo(&mut self) -> Result<(), String> {
        self.stop_playback(false);
        let demo = self.selected_demo().clone();

        match self.audio_engine.render_demo(&demo.key, &demo.path) {
            Ok(render) => {
                let sample_count = render.samples.len();
                let stats = render.stats;
                self.runtime.rendered_audio = Some(render);
                self.sync_selected_track();
                self.set_status(
                    StatusTone::Active,
                    if let Some(stats) = stats {
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
                    },
                );
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
                self.set_status(StatusTone::Warning, message.clone());
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
            self.set_status(StatusTone::Warning, error);
            return;
        }

        let demo_name = self.selected_demo().key.to_uppercase();
        let Some(samples) = self.current_render().map(|render| render.samples.clone()) else {
            self.set_status(
                StatusTone::Warning,
                format!("PLAYBACK ERROR • {} HAS NO PCM", demo_name),
            );
            return;
        };

        match self.playback.start_pcm(samples.as_ref()) {
            Ok(()) => self.set_status(StatusTone::Active, format!("PLAYING • {demo_name}")),
            Err(error) => self.set_status(StatusTone::Warning, format!("PLAYBACK ERROR • {error}")),
        }
    }

    fn stop_playback(&mut self, update_status: bool) {
        match self.playback.stop() {
            Ok(true) if update_status => self.set_status(StatusTone::Normal, "PLAYBACK STOPPED."),
            Ok(_) => {}
            Err(error) if update_status => {
                self.set_status(StatusTone::Warning, format!("STOP ERROR • {error}"))
            }
            Err(_) => {}
        }
    }

    fn poll_playback(&mut self) {
        if let Some(result) = self.playback.poll() {
            match result {
                Ok(()) => self.set_status(StatusTone::Normal, "PLAYBACK FINISHED."),
                Err(error) => {
                    self.set_status(StatusTone::Warning, format!("PLAYBACK ERROR • {error}"))
                }
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Tab) {
                self.cycle_focus(input.modifiers.shift);
            }

            if input.key_pressed(egui::Key::ArrowUp) {
                if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(-1);
                } else {
                    self.move_track_selection(-1);
                }
            }

            if input.key_pressed(egui::Key::ArrowDown) {
                if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(1);
                } else {
                    self.move_track_selection(1);
                }
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

            if input.key_pressed(egui::Key::D) {
                self.focus_panel(FocusArea::DemoBrowser);
            }
            if input.key_pressed(egui::Key::S) {
                self.focus_panel(FocusArea::RenderStats);
            }
            if input.key_pressed(egui::Key::W) {
                self.focus_panel(FocusArea::WaveformView);
            }
            if input.key_pressed(egui::Key::P) {
                self.focus_panel(FocusArea::PatternOverview);
            }
            if input.key_pressed(egui::Key::I) {
                self.focus_panel(FocusArea::InstrumentInspector);
            }
            if input.key_pressed(egui::Key::F) {
                self.focus_panel(FocusArea::FxInspector);
            }
        });
    }

    fn draw_demo_browser(&mut self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            FocusArea::DemoBrowser.title(),
            self.runtime.focus == FocusArea::DemoBrowser,
            Some("UP / DOWN BROWSES DEMOS"),
            |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for index in 0..self.demos.len() {
                    let demo = &self.demos[index];
                    let selected = self.runtime.selected_demo == index;
                    let available = demo.overview.is_some();
                    let label = format!(
                        "{:02}  {}",
                        index + 1,
                        demo.key.replace('_', " ").to_uppercase()
                    );
                    let text_color = if !available {
                        WARNING
                    } else if selected {
                        BASE_BG
                    } else {
                        TEXT
                    };
                    let fill = if selected { ACCENT } else { PANEL_BG };
                    let stroke = if selected {
                        Stroke::new(2.0, ACCENT)
                    } else {
                        Stroke::new(1.0, BORDER_DIM)
                    };

                    let response = ui.add(
                        egui::Button::new(
                            RichText::new(label)
                                .monospace()
                                .size(13.0)
                                .color(text_color),
                        )
                        .fill(fill)
                        .stroke(stroke)
                        .min_size(Vec2::new(ui.available_width(), 24.0)),
                    );

                    if response.clicked() {
                        self.focus_panel(FocusArea::DemoBrowser);
                        self.select_demo(index);
                    }
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                let demo = self.selected_demo();
                ui.label(
                    RichText::new(format!("FILE  {}", demo.path.display()))
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                );

                match &demo.error {
                    Some(error) => {
                        ui.add_space(4.0);
                        ui.label(RichText::new(error).monospace().size(12.0).color(WARNING));
                    }
                    None => {
                        let overview = self.selected_overview();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "TRACKS {} • STEPS {} • BUSES {}",
                                overview.map(|value| value.tracks.len()).unwrap_or(0),
                                overview.map(|value| value.total_steps).unwrap_or(0),
                                overview.map(|value| value.fx_buses.len()).unwrap_or(0)
                            ))
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                        );
                    }
                }
            },
        );
    }

    fn draw_stats_panel(&self, ui: &mut egui::Ui) {
        let demo = self.selected_demo();
        let overview = self.selected_overview();
        let stats = self.current_stats();
        let playback_label = match self.playback.state() {
            PlaybackState::Playing => ("PLAYING", ACCENT),
            PlaybackState::Stopped => ("STOPPED", TEXT_DIM),
            PlaybackState::Error(_) => ("ERROR", WARNING),
        };
        let render_label = if stats.is_some() {
            ("READY", ACCENT)
        } else {
            ("PENDING", TEXT_DIM)
        };

        Self::retro_panel(
            ui,
            FocusArea::RenderStats.title(),
            self.runtime.focus == FocusArea::RenderStats,
            Some("ENTER RENDERS • SPACE STARTS / STOPS"),
            |ui| {
                ui.label(
                    RichText::new(overview.map_or(demo.key.as_str(), |view| view.title.as_str()))
                        .monospace()
                        .size(16.0)
                        .strong(),
                );
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    Self::draw_state_chip(ui, "PLAYBACK", playback_label.0, playback_label.1);
                    Self::draw_state_chip(ui, "RENDER", render_label.0, render_label.1);
                    Self::draw_state_chip(ui, "FOCUS", self.focus_label(), ACCENT);
                });
                ui.add_space(6.0);

                egui::Grid::new("runtime_stats_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(14.0, 6.0))
                    .show(ui, |ui| {
                        Self::grid_row(ui, "demo", &demo.key.to_uppercase(), TEXT);
                        Self::grid_row(
                            ui,
                            "tempo",
                            &overview
                                .map(|value| format!("{} BPM", value.bpm))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "swing",
                            &overview
                                .map(|value| format!("{}%", value.swing_pct))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "steps",
                            &overview
                                .map(|value| {
                                    format!(
                                        "{} @ {} / beat",
                                        value.total_steps, value.steps_per_beat
                                    )
                                })
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "duration",
                            &stats
                                .map(|value| format!("{:.2} ms", value.duration_ms))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "samples",
                            &stats
                                .map(|value| value.sample_count.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "clipping",
                            &stats
                                .map(|value| value.clipping_count.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                            if stats.is_some_and(|value| value.clipping_count > 0) {
                                WARNING
                            } else {
                                TEXT
                            },
                        );
                        Self::grid_row(
                            ui,
                            "peak",
                            &stats
                                .map(|value| value.peak.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "min/max",
                            &stats
                                .map(|value| format!("{} / {}", value.min_sample, value.max_sample))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "render ms",
                            &stats
                                .map(|value| format!("{:.2} ms", value.render_time_ms))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                        Self::grid_row(
                            ui,
                            "checksum",
                            &stats
                                .map(|value| format!("{:016X}", value.checksum))
                                .unwrap_or_else(|| "--".to_string()),
                            TEXT,
                        );
                    });

                ui.add_space(8.0);
                if let Some(overview) = overview {
                    ui.label(
                        RichText::new(format!(
                            "ARRANGEMENT {} • TRACKS {} • HIDDEN {}",
                            overview.arrangement.len(),
                            overview.tracks.len(),
                            overview.hidden_track_count
                        ))
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                    if let Some(track) = self.selected_track() {
                        ui.label(
                            RichText::new(format!(
                                "TRACK {:02} • {} • BUS {}",
                                self.clamped_track_index(overview) + 1,
                                track.name.to_uppercase(),
                                track.fx_bus
                            ))
                            .monospace()
                            .size(12.0)
                            .color(ACCENT),
                        );
                    }
                }
            },
        );
    }

    fn draw_waveform_panel(&self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            FocusArea::WaveformView.title(),
            self.runtime.focus == FocusArea::WaveformView,
            Some("PLAYHEAD FOLLOWS ACTIVE PLAYBACK"),
            |ui| {
                let stats = self.current_stats();
                Self::draw_waveform_minimap(
                    ui,
                    self.current_render().map(|render| render.samples.as_ref()),
                    stats,
                    self.playback_progress(),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(match stats {
                            Some(value) if value.clipping_count > 0 => {
                                format!("CLIP MARKERS {}", value.clipping_count)
                            }
                            Some(_) => "CLIP MARKERS CLEAR".to_string(),
                            None => "RENDER PCM TO ENABLE METERS".to_string(),
                        })
                        .monospace()
                        .size(12.0)
                        .color(
                            if stats.is_some_and(|value| value.clipping_count > 0) {
                                WARNING
                            } else {
                                TEXT_DIM
                            },
                        ),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new("LOW-ALLOCATION ENVELOPE DRAW")
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                });
            },
        );
    }

    fn draw_pattern_panel(&self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            FocusArea::PatternOverview.title(),
            self.runtime.focus == FocusArea::PatternOverview,
            Some("UP / DOWN SELECTS TRACKS"),
            |ui| {
                Self::draw_pattern_visualization(
                    ui,
                    self.selected_overview(),
                    self.runtime.selected_track,
                    self.playback_progress(),
                );
            },
        );
    }

    fn draw_instrument_inspector(&self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            FocusArea::InstrumentInspector.title(),
            self.runtime.focus == FocusArea::InstrumentInspector,
            Some("READ-ONLY VOICE / ADSR VIEW"),
            |ui| {
                let Some(track) = self.selected_track() else {
                    ui.label(
                        RichText::new("NO TRACK METADATA AVAILABLE.")
                            .monospace()
                            .color(WARNING),
                    );
                    return;
                };

                ui.label(
                    RichText::new(format!(
                        "TRACK • {} / {}",
                        track.name.to_uppercase(),
                        track.instrument.to_uppercase()
                    ))
                    .monospace()
                    .strong(),
                );
                if !track.preset.is_empty() {
                    ui.label(
                        RichText::new(format!("PRESET • {}", track.preset.to_uppercase()))
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    Self::draw_waveform_glyph(ui, &track.waveform, track.duty_cycle);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        Self::draw_meter_row(
                            ui,
                            "AMP",
                            track.amplitude as f32 / 127.0,
                            format!("{}", track.amplitude),
                            ACCENT,
                        );
                        Self::draw_meter_row(
                            ui,
                            "DUTY",
                            track.duty_cycle as f32 / 100.0,
                            format!("{}%", track.duty_cycle),
                            ACCENT,
                        );
                        Self::draw_meter_row(
                            ui,
                            "GATE",
                            track.gate_percent as f32 / 100.0,
                            format!("{}%", track.gate_percent),
                            ACCENT,
                        );
                    });
                });
                ui.add_space(8.0);
                Self::draw_adsr_scope(ui, track);
                ui.add_space(8.0);
                Self::draw_meter_row(
                    ui,
                    "GLIDE",
                    (track.glide_ms as f32 / 500.0).clamp(0.0, 1.0),
                    format!("{} ms", track.glide_ms),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "VIBRATO",
                    (track.vibrato_cents as f32 / 32.0).clamp(0.0, 1.0),
                    format!("{} c / {} hz", track.vibrato_cents, track.vibrato_rate),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "DETUNE",
                    (track.detune_cents as f32 / 32.0).clamp(0.0, 1.0),
                    format!("{} c", track.detune_cents),
                    TEXT_DIM,
                );
            },
        );
    }

    fn draw_fx_inspector(&self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            FocusArea::FxInspector.title(),
            self.runtime.focus == FocusArea::FxInspector,
            Some("READ-ONLY BUS / FX LANE VIEW"),
            |ui| {
                let Some(track) = self.selected_track() else {
                    ui.label(
                        RichText::new("NO FX ROUTING AVAILABLE.")
                            .monospace()
                            .color(WARNING),
                    );
                    return;
                };
                let Some(bus) = self.selected_fx_bus() else {
                    ui.label(
                        RichText::new(format!("TRACK ROUTES TO MISSING BUS {}.", track.fx_bus))
                            .monospace()
                            .color(WARNING),
                    );
                    return;
                };

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("BUS {}", bus.bus_index))
                            .monospace()
                            .strong(),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("TRACK {}", track.name.to_uppercase()))
                            .monospace()
                            .color(TEXT_DIM),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(if bus.enabled { "ACTIVE" } else { "BYPASS" })
                            .monospace()
                            .color(if bus.enabled { ACCENT } else { TEXT_DIM }),
                    );
                });
                ui.add_space(8.0);
                Self::draw_meter_row(
                    ui,
                    "DELAY",
                    (bus.delay_mix as f32 / 100.0).clamp(0.0, 1.0),
                    format!(
                        "{} stp / fb {} / mix {}%",
                        bus.delay_steps, bus.delay_feedback, bus.delay_mix
                    ),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "DRIVE",
                    (bus.drive_amount as f32 / 100.0).clamp(0.0, 1.0),
                    format!("{}%", bus.drive_amount),
                    WARNING,
                );
                Self::draw_meter_row(
                    ui,
                    "LOW-PASS",
                    (bus.lowpass_amount as f32 / 100.0).clamp(0.0, 1.0),
                    format!("{}%", bus.lowpass_amount),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "SIDECHAIN",
                    (bus.sidechain_amount as f32 / 100.0).clamp(0.0, 1.0),
                    format!(
                        "{}% / {} ms",
                        bus.sidechain_amount, bus.sidechain_release_ms
                    ),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "BUS MIX",
                    (bus.mix_percent as f32 / 100.0).clamp(0.0, 1.0),
                    format!("{}%", bus.mix_percent),
                    ACCENT,
                );
            },
        );
    }

    fn draw_status_line(&self, ui: &mut egui::Ui) {
        Self::retro_panel(
            ui,
            "STATUS LINE",
            false,
            Some("TAB CYCLES PANELS • I / P / F / W DIRECT"),
            |ui| {
                ui.label(
                    RichText::new(&self.status.text)
                        .monospace()
                        .size(13.0)
                        .color(match self.status.tone {
                            StatusTone::Normal => TEXT,
                            StatusTone::Active => ACCENT,
                            StatusTone::Warning => WARNING,
                        }),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for hint in [
                        "[UP/DOWN] DEMO OR TRACK",
                        "[ENTER] RENDER",
                        "[SPACE] PLAY / STOP",
                        "[ESC] STOP",
                        "[TAB] NEXT PANEL",
                        "[D/S/W/P/I/F] DIRECT FOCUS",
                    ] {
                        ui.label(RichText::new(hint).monospace().size(12.0).color(TEXT_DIM));
                        ui.separator();
                    }
                });
            },
        );
    }

    fn draw_state_chip(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
        egui::Frame::group(ui.style())
            .fill(PANEL_DIM_BG)
            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            .stroke(Stroke::new(1.0, BORDER_DIM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).monospace().size(12.0).color(TEXT_DIM));
                    ui.label(RichText::new(value).monospace().size(12.0).color(color));
                });
            });
    }

    fn grid_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
        ui.label(RichText::new(label).monospace().color(TEXT_DIM));
        ui.label(RichText::new(value).monospace().color(color));
        ui.end_row();
    }

    fn draw_waveform_minimap(
        ui: &mut egui::Ui,
        samples: Option<&[u8]>,
        stats: Option<AudioRenderStats>,
        progress: Option<f32>,
    ) {
        let desired_size = egui::vec2(ui.available_width(), 156.0);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
        let rect = response.rect.shrink(4.0);

        painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));

        let mid_y = rect.center().y;
        painter.line_segment(
            [
                egui::pos2(rect.left(), mid_y),
                egui::pos2(rect.right(), mid_y),
            ],
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
                "RENDER A DEMO TO PREVIEW PCM",
                FontId::monospace(14.0),
                TEXT_DIM,
            );
            return;
        };

        let columns = rect.width().max(64.0) as usize;
        let stride = (samples.len() / columns).max(1);
        let usable_width = (rect.width() - 8.0).max(1.0);
        let mut upper = Vec::with_capacity(columns);
        let mut lower = Vec::with_capacity(columns);
        let mut clip_markers = Vec::with_capacity(columns / 4);

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
                [
                    egui::pos2(x, rect.top() + 4.0),
                    egui::pos2(x, rect.top() + 18.0),
                ],
                Stroke::new(1.5, WARNING),
            );
        }

        if let Some(progress) = progress {
            let x = egui::lerp(rect.left()..=rect.right(), progress.clamp(0.0, 1.0));
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(2.0, ACCENT),
            );
        }

        if let Some(stats) = stats {
            painter.text(
                egui::pos2(rect.left() + 6.0, rect.top() + 6.0),
                Align2::LEFT_TOP,
                format!(
                    "PEAK {} • CLIP {} • {:.2} MS",
                    stats.peak, stats.clipping_count, stats.duration_ms
                ),
                FontId::monospace(11.0),
                if stats.clipping_count > 0 {
                    WARNING
                } else {
                    TEXT_DIM
                },
            );
        }
    }

    fn draw_pattern_visualization(
        ui: &mut egui::Ui,
        overview: Option<&DemoOverview>,
        selected_track: usize,
        progress: Option<f32>,
    ) {
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

        let selected_track = selected_track.min(overview.tracks.len().saturating_sub(1));
        let header_height = 28.0;
        let row_height = 24.0;
        let desired_height = header_height + row_height * overview.tracks.len() as f32 + 12.0;
        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width(), desired_height.max(124.0)),
            Sense::hover(),
        );
        let rect = response.rect.shrink(4.0);
        let label_width = rect.width().clamp(160.0, 240.0);
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

        Self::draw_arrangement_header(
            &painter,
            rect,
            timeline_rect,
            &overview.arrangement,
            total_steps,
            header_height,
        );

        for step_index in 0..=overview.total_steps {
            let is_major = step_index > 0 && step_index % overview.steps_per_beat == 0;
            let x = egui::lerp(
                timeline_rect.left()..=timeline_rect.right(),
                step_index as f32 / total_steps,
            );
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(1.0, if is_major { BORDER_DIM } else { GRID }),
            );
        }

        for (track_index, track) in overview.tracks.iter().enumerate() {
            let row_top = rect.top() + header_height + track_index as f32 * row_height;
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), row_top),
                egui::pos2(rect.right(), row_top + row_height),
            );
            let selected = track_index == selected_track;
            let row_fill = if selected {
                PANEL_ALT_BG
            } else if track_index % 2 == 0 {
                PANEL_DIM_BG
            } else {
                PANEL_BG
            };
            painter.rect_filled(row_rect, 0.0, row_fill);
            painter.line_segment(
                [
                    egui::pos2(rect.left(), row_rect.bottom()),
                    egui::pos2(rect.right(), row_rect.bottom()),
                ],
                Stroke::new(1.0, GRID),
            );
            if selected {
                painter.rect_stroke(row_rect, 0.0, Stroke::new(1.5, ACCENT));
            }

            painter.text(
                egui::pos2(rect.left() + 6.0, row_rect.center().y),
                Align2::LEFT_CENTER,
                format!(
                    "{:02} {} / {}",
                    track_index + 1,
                    track.name,
                    track.instrument
                ),
                FontId::monospace(12.0),
                if selected { ACCENT } else { TEXT },
            );

            for (step_index, step) in track.activity.iter().enumerate() {
                if !step.active || step_index >= overview.total_steps {
                    continue;
                }

                let x0 = egui::lerp(
                    timeline_rect.left()..=timeline_rect.right(),
                    step_index as f32 / total_steps,
                );
                let x1 = egui::lerp(
                    timeline_rect.left()..=timeline_rect.right(),
                    (step_index + 1) as f32 / total_steps,
                );
                let cell_rect = egui::Rect::from_min_max(
                    egui::pos2(x0 + 0.5, row_rect.top() + 4.0),
                    egui::pos2((x1 - 1.0).max(x0 + 1.5), row_rect.bottom() - 4.0),
                );
                let fill = if step.accent { ACCENT_DIM } else { ACCENT_SOFT };
                painter.rect_filled(cell_rect, 0.0, fill);
                painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, ACCENT));
                if step.fx_trigger {
                    painter.line_segment(
                        [
                            egui::pos2(cell_rect.center().x, cell_rect.top()),
                            egui::pos2(cell_rect.center().x, cell_rect.top() + 5.0),
                        ],
                        Stroke::new(1.0, WARNING),
                    );
                }
            }
        }

        if let Some(progress) = progress {
            let x = egui::lerp(
                timeline_rect.left()..=timeline_rect.right(),
                progress.clamp(0.0, 1.0),
            );
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(2.0, ACCENT),
            );
        }
    }

    fn draw_arrangement_header(
        painter: &egui::Painter,
        rect: egui::Rect,
        timeline_rect: egui::Rect,
        arrangement: &[PatternBlock],
        total_steps: f32,
        header_height: f32,
    ) {
        for block in arrangement {
            let x0 = egui::lerp(
                timeline_rect.left()..=timeline_rect.right(),
                block.start_step as f32 / total_steps,
            );
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
                format!("{} {}", block.label, block.length),
                FontId::monospace(11.0),
                ACCENT,
            );
        }
    }

    fn draw_adsr_scope(ui: &mut egui::Ui, track: &TrackOverview) {
        ui.label(RichText::new("ADSR").monospace().size(12.0).color(TEXT_DIM));
        let desired_size = egui::vec2(ui.available_width(), 52.0);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
        let rect = response.rect.shrink(4.0);
        painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));

        let attack = (track.attack_ms as f32 / 350.0).clamp(0.05, 0.35);
        let decay = (track.decay_ms as f32 / 350.0).clamp(0.05, 0.35);
        let release = (track.release_ms as f32 / 450.0).clamp(0.08, 0.4);
        let sustain = (track.sustain_level as f32 / 100.0).clamp(0.0, 1.0);
        let gate = (track.gate_percent as f32 / 100.0).clamp(0.15, 1.0);
        let hold = (1.0 - attack - decay - release).max(0.1) * gate;

        let x0 = rect.left() + 6.0;
        let x1 = egui::lerp(rect.left()..=rect.right(), attack);
        let x2 = egui::lerp(rect.left()..=rect.right(), (attack + decay).min(0.7));
        let x3 = egui::lerp(
            rect.left()..=rect.right(),
            (attack + decay + hold).min(0.88),
        );
        let x4 = rect.right() - 6.0;
        let y_bottom = rect.bottom() - 6.0;
        let y_top = rect.top() + 6.0;
        let y_sustain = egui::lerp(y_bottom..=y_top, sustain);
        let points = vec![
            egui::pos2(x0, y_bottom),
            egui::pos2(x1, y_top),
            egui::pos2(x2, y_sustain),
            egui::pos2(x3, y_sustain),
            egui::pos2(x4, y_bottom),
        ];
        painter.add(egui::Shape::line(points, Stroke::new(1.5, ACCENT)));
    }

    fn draw_waveform_glyph(ui: &mut egui::Ui, waveform: &str, duty_cycle: i32) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("WAVEFORM")
                    .monospace()
                    .size(12.0)
                    .color(TEXT_DIM),
            );
            let (response, painter) = ui.allocate_painter(egui::vec2(90.0, 40.0), Sense::hover());
            let rect = response.rect.shrink(4.0);
            painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));

            let points = match waveform {
                "triangle" => vec![
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
                    egui::pos2(rect.center().x, rect.top() + 4.0),
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 4.0),
                ],
                "noise" => vec![
                    egui::pos2(rect.left() + 4.0, rect.center().y),
                    egui::pos2(rect.left() + 18.0, rect.top() + 6.0),
                    egui::pos2(rect.left() + 28.0, rect.bottom() - 6.0),
                    egui::pos2(rect.left() + 42.0, rect.top() + 8.0),
                    egui::pos2(rect.left() + 58.0, rect.bottom() - 5.0),
                    egui::pos2(rect.right() - 4.0, rect.center().y),
                ],
                "pulse" => {
                    let width = rect.width() * (duty_cycle as f32 / 100.0).clamp(0.1, 0.9);
                    vec![
                        egui::pos2(rect.left() + 4.0, rect.bottom() - 5.0),
                        egui::pos2(rect.left() + 4.0, rect.top() + 5.0),
                        egui::pos2(rect.left() + width, rect.top() + 5.0),
                        egui::pos2(rect.left() + width, rect.bottom() - 5.0),
                        egui::pos2(rect.right() - 4.0, rect.bottom() - 5.0),
                    ]
                }
                _ => vec![
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 5.0),
                    egui::pos2(rect.left() + 4.0, rect.top() + 5.0),
                    egui::pos2(rect.center().x, rect.top() + 5.0),
                    egui::pos2(rect.center().x, rect.bottom() - 5.0),
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 5.0),
                ],
            };
            painter.add(egui::Shape::line(points, Stroke::new(1.5, ACCENT)));
            ui.label(
                RichText::new(waveform.to_uppercase())
                    .monospace()
                    .size(11.0)
                    .color(TEXT),
            );
        });
    }

    fn draw_meter_row(ui: &mut egui::Ui, label: &str, fraction: f32, value: String, fill: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).monospace().size(12.0).color(TEXT_DIM));
            let bar_width = (ui.available_width() - 84.0).max(72.0);
            let (response, painter) =
                ui.allocate_painter(egui::vec2(bar_width, 12.0), Sense::hover());
            let rect = response.rect.shrink(1.0);
            painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));
            if fraction > 0.0 {
                let fill_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(
                        egui::lerp(rect.left()..=rect.right(), fraction.clamp(0.0, 1.0)),
                        rect.bottom(),
                    ),
                );
                painter.rect_filled(fill_rect, 0.0, fill);
            }
            ui.label(RichText::new(value).monospace().size(11.0).color(TEXT));
        });
    }

    fn retro_panel<R>(
        ui: &mut egui::Ui,
        title: &str,
        focused: bool,
        subtitle: Option<&str>,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let frame = egui::Frame::group(ui.style())
            .fill(PANEL_BG)
            .inner_margin(egui::Margin::same(10.0))
            .stroke(Stroke::new(
                if focused { 2.0 } else { 1.0 },
                if focused { ACCENT } else { BORDER },
            ));

        frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(title)
                            .monospace()
                            .size(14.0)
                            .strong()
                            .color(if focused { ACCENT } else { TEXT }),
                    );
                    if focused {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("FOCUSED")
                                    .monospace()
                                    .size(11.0)
                                    .color(ACCENT),
                            );
                        });
                    }
                });
                if let Some(subtitle) = subtitle {
                    ui.label(
                        RichText::new(subtitle)
                            .monospace()
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                }
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
        self.sync_selected_track();

        if self.playback.is_playing() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }

        egui::TopBottomPanel::top("runtime_header")
            .exact_height(58.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("MEMDECK SOUND MACHINE");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("FOCUS • {}", self.focus_label()))
                                .monospace()
                                .color(ACCENT),
                        );
                    });
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "DEMO • {}",
                            self.selected_demo().key.to_uppercase()
                        ))
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                    if let Some(track) = self.selected_track() {
                        ui.separator();
                        ui.label(
                            RichText::new(format!("TRACK • {}", track.name.to_uppercase()))
                                .monospace()
                                .size(12.0)
                                .color(TEXT_DIM),
                        );
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "BOOT • {}",
                            self.runtime.focus.boot_key().to_uppercase()
                        ))
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                });
            });

        egui::TopBottomPanel::bottom("runtime_footer")
            .resizable(false)
            .show(ctx, |ui| self.draw_status_line(ui));

        egui::SidePanel::left("demo_browser")
            .resizable(false)
            .exact_width(286.0)
            .show(ctx, |ui| self.draw_demo_browser(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.columns(2, |columns| {
                    self.draw_stats_panel(&mut columns[0]);
                    self.draw_waveform_panel(&mut columns[1]);
                });
                ui.add_space(8.0);
                self.draw_pattern_panel(ui);
                ui.add_space(8.0);
                ui.columns(2, |columns| {
                    self.draw_instrument_inspector(&mut columns[0]);
                    self.draw_fx_inspector(&mut columns[1]);
                });
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
                Some("overview") | Some("stats") => Some(FocusArea::RenderStats),
                Some("demos") => Some(FocusArea::DemoBrowser),
                Some("waveform") | Some("audio") => Some(FocusArea::WaveformView),
                Some("pattern") | Some("arrangement") => Some(FocusArea::PatternOverview),
                Some("instrument") | Some("inspector") => Some(FocusArea::InstrumentInspector),
                Some("fx") => Some(FocusArea::FxInspector),
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
