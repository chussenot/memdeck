use std::env;
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke, TextStyle, Vec2};

use crate::audio_engine::{
    AudioRenderStats, DemoEntry, DemoOverview, FxBusOverview, GuiAudioEngine, PatternBlock,
    RenderState, TrackOverview,
};
use crate::editor::{
    self, EditableArrangementBlock, EditablePattern, EditableSong, EditorMode, EditorState,
};
use crate::ffi;
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
const PATTERN_EDITOR_GATE_STEPS: [u8; 5] = [25, 50, 75, 90, 100];
const PATTERN_EDITOR_VELOCITY_STEPS: [u8; 5] = [32, 64, 88, 110, 127];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusArea {
    DemoBrowser,
    RenderStats,
    WaveformView,
    PatternOverview,
    PatternEditor,
    InstrumentInspector,
    FxInspector,
}

impl FocusArea {
    const ALL: [FocusArea; 7] = [
        FocusArea::DemoBrowser,
        FocusArea::RenderStats,
        FocusArea::WaveformView,
        FocusArea::PatternOverview,
        FocusArea::PatternEditor,
        FocusArea::InstrumentInspector,
        FocusArea::FxInspector,
    ];

    fn title(self) -> &'static str {
        match self {
            FocusArea::DemoBrowser => "DEMO BROWSER",
            FocusArea::RenderStats => "RENDER STATS",
            FocusArea::WaveformView => "WAVEFORM",
            FocusArea::PatternOverview => "PATTERN OVERVIEW",
            FocusArea::PatternEditor => "PATTERN EDITOR",
            FocusArea::InstrumentInspector => "INSTRUMENT INSPECTOR",
            FocusArea::FxInspector => "FX INSPECTOR",
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
    editable_source: Option<String>,
    mode: Option<EditorMode>,
    apply_pattern_edits: bool,
}

struct RuntimeState {
    selected_demo: usize,
    selected_track: usize,
    rendered_audio: Option<RenderState>,
    focus: FocusArea,
    last_error: Option<String>,
}

pub struct MemDeckGuiApp {
    audio_engine: GuiAudioEngine,
    playback: PlaybackController,
    demos: Vec<DemoEntry>,
    runtime: RuntimeState,
    status: StatusMessage,
    boot_options: BootOptions,
    boot_applied: bool,
    editor_state: EditorState,
    editable_song: Option<EditableSong>,
    editor_open_path: String,
    renaming_pattern: bool,
    pattern_rename_buffer: String,
    step_clipboard: Option<editor::EditableStep>,
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
                last_error: None,
            },
            status,
            boot_options,
            boot_applied: false,
            editor_state: EditorState::default(),
            editable_song: None,
            editor_open_path: String::new(),
            renaming_pattern: false,
            pattern_rename_buffer: String::new(),
            step_clipboard: None,
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
        let render = self
            .runtime
            .rendered_audio
            .as_ref()
            ?;
        if self.editor_state.mode != EditorMode::Browser && render.demo_key == "__editable__" {
            return Some(render);
        }
        if render.demo_key == self.selected_demo().key {
            return Some(render);
        }
        None
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
        let message = message.into();
        if tone == StatusTone::Warning {
            self.runtime.last_error = Some(message.clone());
        }
        self.status = StatusMessage {
            text: message,
            tone,
        };
    }

    fn apply_boot_options(&mut self) {
        if self.boot_applied {
            return;
        }

        match self.boot_options.editable_source.as_deref() {
            Some("new") => self.create_new_song(),
            Some("duplicate") => self.duplicate_demo_as_editable(),
            _ => {}
        }

        if self.boot_options.apply_pattern_edits {
            self.apply_boot_pattern_edits();
        }

        if let Some(mode) = self.boot_options.mode {
            match mode {
                EditorMode::Browser => self.set_mode(EditorMode::Browser),
                EditorMode::Edit => {
                    if self.editable_song.is_some() {
                        self.set_mode(EditorMode::Edit);
                    }
                }
                EditorMode::Preview => {
                    if self.editable_song.is_some() {
                        self.render_editable_preview();
                    }
                }
            }
        }

        if self.boot_options.auto_render {
            if self.editor_state.mode == EditorMode::Browser {
                let _ = self.render_selected_demo();
            } else {
                self.render_editable_preview();
            }
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
        self.runtime.rendered_audio = None;

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

    fn set_mode(&mut self, mode: EditorMode) {
        self.editor_state.mode = mode;
    }

    fn create_new_song(&mut self) {
        self.stop_playback(false);
        self.editable_song = Some(EditableSong::new_song());
        self.editor_state = EditorState {
            mode: EditorMode::Edit,
            selected_pattern: Some(0),
            selected_track: 0,
            selected_step: Some(0),
            selected_arrangement_block: Some(0),
            dirty: false,
            last_saved_path: None,
            last_error: None,
        };
        self.renaming_pattern = false;
        self.pattern_rename_buffer.clear();
        self.set_status(StatusTone::Active, "EDIT MODE • NEW SONG");
    }

    fn duplicate_demo_as_editable(&mut self) {
        let path = self.selected_demo().path.clone();
        match editor::load_editable_song_from_path(&path) {
            Ok(mut song) => {
                song.source_path = None;
                song.dirty = true;
                self.stop_playback(false);
                self.editable_song = Some(song);
                self.editor_state.mode = EditorMode::Edit;
                self.editor_state.selected_arrangement_block = Some(0);
                self.editor_state.selected_pattern = Some(0);
                self.editor_state.selected_step = Some(0);
                self.editor_state.selected_track = 0;
                self.editor_state.dirty = true;
                self.editor_state.last_error = None;
                self.set_status(StatusTone::Active, "EDIT MODE • DEMO DUPLICATED");
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("DUPLICATE FAILED • {error}"));
            }
        }
    }

    fn open_editable_song_from_path(&mut self) {
        let path = PathBuf::from(self.editor_open_path.trim());
        if self.editor_open_path.trim().is_empty() {
            self.set_status(StatusTone::Warning, "OPEN FAILED • ENTER A FILE PATH");
            return;
        }

        match editor::load_editable_song_from_path(&path) {
            Ok(song) => {
                self.stop_playback(false);
                self.editable_song = Some(song);
                self.editor_state.mode = EditorMode::Edit;
                self.editor_state.selected_arrangement_block = Some(0);
                self.editor_state.selected_pattern = Some(0);
                self.editor_state.selected_step = Some(0);
                self.editor_state.selected_track = 0;
                self.editor_state.dirty = false;
                self.editor_state.last_saved_path = Some(path.clone());
                self.editor_state.last_error = None;
                self.set_status(
                    StatusTone::Active,
                    format!("EDIT MODE • OPENED {}", path.display()),
                );
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("OPEN FAILED • {error}"));
            }
        }
    }

    fn save_editable_song(&mut self, save_as: bool) {
        let Some(song) = self.editable_song.as_mut() else {
            self.set_status(StatusTone::Warning, "SAVE FAILED • NO EDITABLE SONG");
            return;
        };

        let target_path = if save_as {
            let input = self.editor_open_path.trim();
            if input.is_empty() {
                self.set_status(StatusTone::Warning, "SAVE AS FAILED • ENTER A FILE PATH");
                return;
            }
            PathBuf::from(input)
        } else {
            song.source_path
                .clone()
                .or_else(|| self.editor_state.last_saved_path.clone())
                .unwrap_or_else(|| PathBuf::from(self.editor_open_path.trim()))
        };

        if target_path.as_os_str().is_empty() {
            self.set_status(StatusTone::Warning, "SAVE FAILED • NO TARGET FILE");
            return;
        }

        match editor::save_editable_song_to_path(song, &target_path) {
            Ok(()) => {
                self.editor_state.dirty = false;
                self.editor_state.last_saved_path = Some(target_path.clone());
                self.editor_state.clear_error();
                self.set_status(
                    StatusTone::Active,
                    format!("SAVED • {}", target_path.display()),
                );
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("SAVE FAILED • {error}"));
            }
        }
    }

    fn render_editable_preview(&mut self) {
        let Some(song) = self.editable_song.as_ref() else {
            self.set_status(StatusTone::Warning, "PREVIEW FAILED • NO EDITABLE SONG");
            return;
        };

        let abc = match editor::serialize_editable_song(song) {
            Ok(abc) => abc,
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("PREVIEW FAILED • {error}"));
                return;
            }
        };

        let tmp_path = std::env::temp_dir().join(format!(
            "memdeck-edit-preview-{}.abc",
            std::process::id()
        ));
        if let Err(error) = std::fs::write(&tmp_path, abc) {
            self.set_status(
                StatusTone::Warning,
                format!("PREVIEW FAILED • WRITE ERROR • {error}"),
            );
            return;
        }

        match ffi::render_abc_file(&tmp_path) {
            Ok(samples) => {
                self.runtime.rendered_audio = Some(RenderState {
                    demo_key: "__editable__".to_string(),
                    samples: std::sync::Arc::<[u8]>::from(samples),
                    stats: ffi::get_render_stats(),
                });
                self.set_mode(EditorMode::Preview);
                self.set_status(StatusTone::Active, "PREVIEW RENDERED");
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("PREVIEW FAILED • {error}"));
            }
        }

        let _ = std::fs::remove_file(tmp_path);
    }

    fn arrangement_move_cursor(&mut self, delta: isize) {
        let Some(song) = self.editable_song.as_ref() else {
            return;
        };
        let block_count = song.arrangement.blocks.len();
        if block_count == 0 {
            self.editor_state.selected_arrangement_block = None;
            return;
        }
        let current = self.editor_state.selected_arrangement_block.unwrap_or(0);
        let next = (current as isize + delta).clamp(0, block_count.saturating_sub(1) as isize);
        self.editor_state.selected_arrangement_block = Some(next as usize);
    }

    fn arrangement_add_block(&mut self) {
        let current_index = self.editor_state.selected_arrangement_block.unwrap_or(0);
        let new_selected = {
            let Some(song) = self.editable_song.as_mut() else {
                return;
            };
            let next_pattern_name = format!("P{}", song.patterns.len() + 1);
            song.patterns.push(EditablePattern {
                name: next_pattern_name.clone(),
                length: 16,
            });
            let insert_at = current_index + 1;
            let block = EditableArrangementBlock {
                pattern_name: next_pattern_name,
                length: 16,
            };
            let selected = if insert_at >= song.arrangement.blocks.len() {
                song.arrangement.blocks.push(block);
                Some(song.arrangement.blocks.len().saturating_sub(1))
            } else {
                song.arrangement.blocks.insert(insert_at, block);
                Some(insert_at)
            };
            song.mark_dirty();
            selected
        };
        self.editor_state.selected_arrangement_block = new_selected;
        self.editor_state.dirty = true;
    }

    fn arrangement_duplicate_block(&mut self) {
        let selected = self.editor_state.selected_arrangement_block;
        let (new_selected, changed) = {
            let Some(song) = self.editable_song.as_mut() else {
                return;
            };
            let Some(index) = selected else {
                return;
            };
            if let Some(block) = song.arrangement.blocks.get(index).cloned() {
                let insert_at = index + 1;
                song.arrangement.blocks.insert(insert_at, block);
                song.mark_dirty();
                (Some(insert_at), true)
            } else {
                (selected, false)
            }
        };
        self.editor_state.selected_arrangement_block = new_selected;
        if changed {
            self.editor_state.dirty = true;
        }
    }

    fn arrangement_remove_block(&mut self) {
        let selected = self.editor_state.selected_arrangement_block;
        let (new_selected, changed) = {
            let Some(song) = self.editable_song.as_mut() else {
                return;
            };
            if song.arrangement.blocks.len() <= 1 {
                return;
            }
            let Some(index) = selected else {
                return;
            };
            if index < song.arrangement.blocks.len() {
                song.arrangement.blocks.remove(index);
                song.mark_dirty();
                (
                    Some(index.min(song.arrangement.blocks.len().saturating_sub(1))),
                    true,
                )
            } else {
                (selected, false)
            }
        };
        self.editor_state.selected_arrangement_block = new_selected;
        if changed {
            self.editor_state.dirty = true;
        }
    }

    fn arrangement_reorder_selected(&mut self, delta: isize) {
        let selected = self.editor_state.selected_arrangement_block;
        let new_selected = {
            let Some(song) = self.editable_song.as_mut() else {
                return;
            };
            let Some(index) = selected else {
                return;
            };
            let target = (index as isize + delta)
                .clamp(0, song.arrangement.blocks.len().saturating_sub(1) as isize)
                as usize;
            if index == target {
                return;
            }
            song.arrangement.blocks.swap(index, target);
            song.mark_dirty();
            Some(target)
        };
        self.editor_state.selected_arrangement_block = new_selected;
        self.editor_state.dirty = true;
    }

    fn begin_rename_selected_pattern(&mut self) {
        let Some(song) = self.editable_song.as_ref() else {
            return;
        };
        let Some(index) = self.editor_state.selected_arrangement_block else {
            return;
        };
        let Some(block) = song.arrangement.blocks.get(index) else {
            return;
        };
        self.renaming_pattern = true;
        self.pattern_rename_buffer = block.pattern_name.clone();
    }

    fn apply_pattern_rename(&mut self) {
        let new_name = self.pattern_rename_buffer.trim().to_string();
        if new_name.is_empty() {
            self.renaming_pattern = false;
            self.pattern_rename_buffer.clear();
            return;
        }

        let selected = self.editor_state.selected_arrangement_block;
        let Some(song) = self.editable_song.as_mut() else {
            return;
        };
        let Some(index) = selected else {
            return;
        };
        let Some(previous_name) = song
            .arrangement
            .blocks
            .get(index)
            .map(|block| block.pattern_name.clone())
        else {
            return;
        };

        for pattern in &mut song.patterns {
            if pattern.name == previous_name {
                pattern.name = new_name.clone();
            }
        }
        for block in &mut song.arrangement.blocks {
            if block.pattern_name == previous_name {
                block.pattern_name = new_name.clone();
            }
        }
        song.mark_dirty();
        self.editor_state.dirty = true;
        self.renaming_pattern = false;
        self.pattern_rename_buffer.clear();
    }

    fn cancel_current_edit(&mut self) {
        if self.renaming_pattern {
            self.renaming_pattern = false;
            self.pattern_rename_buffer.clear();
            self.set_status(StatusTone::Normal, "RENAME CANCELED");
            return;
        }
        if self.editor_state.mode != EditorMode::Browser && self.runtime.focus == FocusArea::PatternEditor {
            self.focus_panel(FocusArea::PatternOverview);
            return;
        }
        self.stop_playback(true);
    }

    fn open_selected_pattern(&mut self) {
        let Some(block_index) = self.editor_state.selected_arrangement_block else {
            return;
        };
        let Some((block_name, selected_pattern, block_start)) = self.editable_song.as_ref().and_then(|song| {
            let block = song.arrangement.blocks.get(block_index)?;
            let selected_pattern = song
                .patterns
                .iter()
                .position(|pattern| pattern.name == block.pattern_name);
            let block_start = song
                .arrangement
                .blocks
                .iter()
                .take(block_index)
                .map(|b| b.length)
                .sum::<usize>();
            Some((block.pattern_name.clone(), selected_pattern, block_start))
        }) else {
            return;
        };
        self.editor_state.selected_pattern = selected_pattern;
        self.editor_state.selected_step = Some(block_start);
        self.focus_panel(FocusArea::PatternEditor);
        self.set_status(
            StatusTone::Normal,
            format!("PATTERN OPEN • {}", block_name.to_uppercase()),
        );
    }

    fn current_pattern_bounds(&self) -> Option<(usize, usize, usize)> {
        let song = self.editable_song.as_ref()?;
        let block_index = self.editor_state.selected_arrangement_block.unwrap_or(0);
        let block = song.arrangement.blocks.get(block_index)?;
        let mut start = 0usize;
        for prior in song.arrangement.blocks.iter().take(block_index) {
            start += prior.length;
        }
        Some((block_index, start, block.length.max(1)))
    }

    fn normalize_pattern_cursor(&mut self) {
        let Some(song) = self.editable_song.as_ref() else {
            self.editor_state.selected_step = None;
            return;
        };
        if song.tracks.is_empty() {
            self.editor_state.selected_track = 0;
            self.editor_state.selected_step = None;
            return;
        }
        self.editor_state.selected_track = self
            .editor_state
            .selected_track
            .min(song.tracks.len().saturating_sub(1));
        self.runtime.selected_track = self.editor_state.selected_track;

        let Some((_, block_start, block_len)) = self.current_pattern_bounds() else {
            self.editor_state.selected_step = None;
            return;
        };
        let clamped = self
            .editor_state
            .selected_step
            .unwrap_or(block_start)
            .clamp(block_start, block_start + block_len.saturating_sub(1));
        self.editor_state.selected_step = Some(clamped);
    }

    fn move_pattern_cursor(&mut self, delta_track: isize, delta_step: isize) {
        self.normalize_pattern_cursor();
        let Some(song) = self.editable_song.as_ref() else {
            return;
        };
        if song.tracks.is_empty() {
            return;
        }
        let Some((_, block_start, block_len)) = self.current_pattern_bounds() else {
            return;
        };
        let track_max = song.tracks.len().saturating_sub(1) as isize;
        let next_track = (self.editor_state.selected_track as isize + delta_track).clamp(0, track_max);
        self.editor_state.selected_track = next_track as usize;
        self.runtime.selected_track = self.editor_state.selected_track;

        let relative = self
            .editor_state
            .selected_step
            .unwrap_or(block_start)
            .saturating_sub(block_start) as isize;
        let step_max = block_len.saturating_sub(1) as isize;
        let next_relative = (relative + delta_step).clamp(0, step_max);
        self.editor_state.selected_step = Some(block_start + next_relative as usize);
    }

    fn set_pattern_cell(&mut self, track_index: usize, relative_step: usize) {
        let Some((_, block_start, block_len)) = self.current_pattern_bounds() else {
            return;
        };
        let Some(song) = self.editable_song.as_ref() else {
            return;
        };
        if song.tracks.is_empty() {
            return;
        }
        self.editor_state.selected_track = track_index.min(song.tracks.len().saturating_sub(1));
        self.runtime.selected_track = self.editor_state.selected_track;
        self.editor_state.selected_step =
            Some(block_start + relative_step.min(block_len.saturating_sub(1)));
    }

    fn mark_editor_dirty(&mut self) {
        if let Some(song) = self.editable_song.as_mut() {
            song.mark_dirty();
        }
        self.editor_state.dirty = true;
    }

    fn with_selected_step_mut(
        &mut self,
        mutate: impl FnOnce(&mut editor::EditableStep),
    ) -> bool {
        self.normalize_pattern_cursor();
        let Some(song) = self.editable_song.as_mut() else {
            return false;
        };
        let Some(step_index) = self.editor_state.selected_step else {
            return false;
        };
        let Some(track) = song.tracks.get_mut(self.editor_state.selected_track) else {
            return false;
        };
        if step_index >= track.steps.len() {
            return false;
        }
        mutate(&mut track.steps[step_index]);
        true
    }

    fn toggle_selected_step(&mut self) {
        if self.with_selected_step_mut(|step| step.toggle_active()) {
            self.mark_editor_dirty();
        }
    }

    fn adjust_selected_step_octave(&mut self, semitone_delta: i16) {
        if self.with_selected_step_mut(|step| {
            if !step.active {
                step.toggle_active();
            }
            let current = if step.midi_note == 0 {
                editor::EditableStep::DEFAULT_MIDI_NOTE
            } else {
                step.midi_note
            } as i16;
            step.midi_note = (current + semitone_delta).clamp(1, 127) as u8;
            step.active = true;
        }) {
            self.mark_editor_dirty();
        }
    }

    fn toggle_selected_step_accent(&mut self) {
        if self.with_selected_step_mut(|step| step.accent = !step.accent) {
            self.mark_editor_dirty();
        }
    }

    fn toggle_selected_step_fx_trigger(&mut self) {
        if self.with_selected_step_mut(|step| step.fx_trigger = !step.fx_trigger) {
            self.mark_editor_dirty();
        }
    }

    fn cycle_selected_step_gate(&mut self) {
        if self.with_selected_step_mut(|step| {
            let pos = PATTERN_EDITOR_GATE_STEPS
                .iter()
                .position(|value| *value == step.gate_percent)
                .unwrap_or(0);
            step.gate_percent =
                PATTERN_EDITOR_GATE_STEPS[(pos + 1) % PATTERN_EDITOR_GATE_STEPS.len()];
        }) {
            self.mark_editor_dirty();
        }
    }

    fn cycle_selected_step_velocity(&mut self) {
        if self.with_selected_step_mut(|step| {
            let pos = PATTERN_EDITOR_VELOCITY_STEPS
                .iter()
                .position(|value| *value == step.velocity)
                .unwrap_or(2);
            step.velocity =
                PATTERN_EDITOR_VELOCITY_STEPS[(pos + 1) % PATTERN_EDITOR_VELOCITY_STEPS.len()];
        }) {
            self.mark_editor_dirty();
        }
    }

    fn copy_selected_step(&mut self, cut: bool) {
        self.normalize_pattern_cursor();
        let Some(song) = self.editable_song.as_mut() else {
            return;
        };
        let Some(step_index) = self.editor_state.selected_step else {
            return;
        };
        let Some(track) = song.tracks.get_mut(self.editor_state.selected_track) else {
            return;
        };
        let Some(step) = track.steps.get(step_index).cloned() else {
            return;
        };
        self.step_clipboard = Some(step);
        if cut {
            if let Some(step_mut) = track.steps.get_mut(step_index) {
                *step_mut = editor::EditableStep::rest();
                song.mark_dirty();
                self.editor_state.dirty = true;
            }
        }
    }

    fn paste_selected_step(&mut self) {
        let Some(clipboard_step) = self.step_clipboard.clone() else {
            return;
        };
        if self.with_selected_step_mut(|step| *step = clipboard_step) {
            self.mark_editor_dirty();
        }
    }

    fn apply_boot_pattern_edits(&mut self) {
        if self.editable_song.is_none() {
            return;
        }
        self.editor_state.mode = EditorMode::Edit;
        self.editor_state.selected_arrangement_block = Some(0);
        self.editor_state.selected_track = 0;
        self.editor_state.selected_step = Some(0);
        self.toggle_selected_step();
        self.adjust_selected_step_octave(12);
        self.toggle_selected_step_accent();
        self.toggle_selected_step_fx_trigger();
        self.cycle_selected_step_velocity();
        self.cycle_selected_step_gate();
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
            if input.modifiers.command && input.modifiers.shift && input.key_pressed(egui::Key::S) {
                self.save_editable_song(true);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::S) && !input.modifiers.shift {
                self.save_editable_song(false);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::R) {
                self.render_editable_preview();
            }

            if input.key_pressed(egui::Key::Tab) {
                self.handle_key_press(egui::Key::Tab, input.modifiers.shift);
            }

            for key in [
                egui::Key::ArrowLeft,
                egui::Key::ArrowRight,
                egui::Key::ArrowUp,
                egui::Key::ArrowDown,
                egui::Key::Enter,
                egui::Key::Space,
                egui::Key::Escape,
                egui::Key::Delete,
                egui::Key::Backspace,
                egui::Key::Plus,
                egui::Key::Minus,
                egui::Key::A,
                egui::Key::C,
                egui::Key::D,
                egui::Key::E,
                egui::Key::G,
                egui::Key::N,
                egui::Key::R,
                egui::Key::S,
                egui::Key::V,
                egui::Key::W,
                egui::Key::X,
                egui::Key::P,
                egui::Key::I,
                egui::Key::F,
            ] {
                if input.key_pressed(key) {
                    self.handle_key_press_with_modifiers(key, input.modifiers.shift, input.modifiers.command);
                }
            }
        });
    }

    fn handle_key_press(&mut self, key: egui::Key, shift: bool) {
        self.handle_key_press_with_modifiers(key, shift, false);
    }

    fn handle_key_press_with_modifiers(
        &mut self,
        key: egui::Key,
        shift: bool,
        command: bool,
    ) {
        match key {
            egui::Key::Tab => self.cycle_focus(shift),
            egui::Key::ArrowLeft => {
                if self.editor_state.mode != EditorMode::Browser && self.runtime.focus == FocusArea::PatternEditor {
                    self.move_pattern_cursor(0, -1);
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    if command {
                        self.arrangement_reorder_selected(-1);
                    } else {
                        self.arrangement_move_cursor(-1);
                    }
                }
            }
            egui::Key::ArrowRight => {
                if self.editor_state.mode != EditorMode::Browser && self.runtime.focus == FocusArea::PatternEditor {
                    self.move_pattern_cursor(0, 1);
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    if command {
                        self.arrangement_reorder_selected(1);
                    } else {
                        self.arrangement_move_cursor(1);
                    }
                }
            }
            egui::Key::ArrowUp => {
                if self.editor_state.mode != EditorMode::Browser && self.runtime.focus == FocusArea::PatternEditor {
                    self.move_pattern_cursor(-1, 0);
                } else if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(-1);
                } else {
                    self.move_track_selection(-1);
                    self.editor_state.selected_track = self.runtime.selected_track;
                }
            }
            egui::Key::ArrowDown => {
                if self.editor_state.mode != EditorMode::Browser && self.runtime.focus == FocusArea::PatternEditor {
                    self.move_pattern_cursor(1, 0);
                } else if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(1);
                } else {
                    self.move_track_selection(1);
                    self.editor_state.selected_track = self.runtime.selected_track;
                }
            }
            egui::Key::Enter => {
                if self.renaming_pattern {
                    self.apply_pattern_rename();
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.toggle_selected_step();
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    self.open_selected_pattern();
                } else {
                    let _ = self.render_selected_demo();
                }
            }
            egui::Key::Space => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.toggle_selected_step();
                } else {
                    self.toggle_playback();
                }
            }
            egui::Key::Escape => self.cancel_current_edit(),
            egui::Key::Delete | egui::Key::Backspace => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    self.arrangement_remove_block();
                }
            }
            egui::Key::Plus => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.adjust_selected_step_octave(12);
                }
            }
            egui::Key::Minus => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.adjust_selected_step_octave(-12);
                }
            }
            egui::Key::A => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.toggle_selected_step_accent();
                } else {
                    self.focus_panel(FocusArea::PatternOverview)
                }
            }
            egui::Key::C => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.copy_selected_step(false);
                }
            }
            egui::Key::D => {
                if command {
                    // Keep Cmd/Ctrl+D available to the host platform and avoid conflicting
                    // with arrangement duplicate semantics.
                    return;
                }
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    self.arrangement_duplicate_block();
                } else {
                    self.focus_panel(FocusArea::DemoBrowser);
                }
            }
            egui::Key::E => self.focus_panel(FocusArea::PatternEditor),
            egui::Key::G => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.cycle_selected_step_gate();
                } else {
                    self.focus_panel(FocusArea::PatternEditor);
                }
            }
            egui::Key::N => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    self.arrangement_add_block();
                }
            }
            egui::Key::R => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                    && !command
                {
                    self.begin_rename_selected_pattern();
                }
            }
            egui::Key::S => {
                if !command {
                    self.focus_panel(FocusArea::RenderStats)
                }
            }
            egui::Key::W => {
                if !command {
                    self.focus_panel(FocusArea::WaveformView)
                }
            }
            egui::Key::P => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.paste_selected_step();
                } else if !command {
                    self.focus_panel(FocusArea::PatternOverview)
                }
            }
            egui::Key::V => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.cycle_selected_step_velocity();
                }
            }
            egui::Key::I => {
                if !command {
                    self.focus_panel(FocusArea::InstrumentInspector)
                }
            }
            egui::Key::F => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.toggle_selected_step_fx_trigger();
                } else if !command {
                    self.focus_panel(FocusArea::FxInspector)
                }
            }
            egui::Key::X => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.copy_selected_step(true);
                }
            }
            _ => {}
        }
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
            ("IDLE", TEXT_DIM)
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
                            None => "RENDER PCM TO VIEW WAVEFORM".to_string(),
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

    fn draw_pattern_panel(&mut self, ui: &mut egui::Ui) {
        let in_edit_mode = self.editor_state.mode != EditorMode::Browser;
        Self::retro_panel(
            ui,
            if in_edit_mode {
                "ARRANGEMENT EDITOR"
            } else {
                FocusArea::PatternOverview.title()
            },
            self.runtime.focus == FocusArea::PatternOverview,
            Some(if in_edit_mode {
                "A FOCUSES • N NEW • D DUPLICATE • DEL REMOVE • R RENAME • CTRL+S SAVE"
            } else {
                "UP / DOWN SELECTS TRACKS"
            }),
            |ui| {
                if in_edit_mode {
                    self.draw_arrangement_editor(ui);
                } else {
                    Self::draw_pattern_visualization(
                        ui,
                        self.selected_overview(),
                        self.runtime.selected_track,
                        self.playback_progress(),
                    );
                }
            },
        );
    }

    fn draw_pattern_editor_panel(&mut self, ui: &mut egui::Ui) {
        let in_edit_mode = self.editor_state.mode != EditorMode::Browser;
        Self::retro_panel(
            ui,
            FocusArea::PatternEditor.title(),
            self.runtime.focus == FocusArea::PatternEditor,
            Some(if in_edit_mode {
                "ARROWS MOVE • ENTER/SPACE TOGGLE • +/- OCTAVE • A/F ACCENT+FX • G/V GATE+VEL • C/X/P CLIPBOARD • ESC EXIT"
            } else {
                "EDIT MODE REQUIRED"
            }),
            |ui| {
                if !in_edit_mode {
                    ui.label(
                        RichText::new("BROWSER MODE IS READ-ONLY. USE NEW/DUPLICATE/OPEN TO EDIT.")
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                    return;
                }
                self.draw_pattern_editor_grid(ui);
            },
        );
    }

    fn draw_pattern_editor_grid(&mut self, ui: &mut egui::Ui) {
        self.normalize_pattern_cursor();
        let Some(song) = self.editable_song.as_ref() else {
            ui.label(
                RichText::new("NO EDITABLE SONG. USE NEW SONG / DUPLICATE / OPEN.")
                    .monospace()
                    .color(WARNING),
            );
            return;
        };
        let Some((block_index, block_start, block_len)) = self.current_pattern_bounds() else {
            ui.label(RichText::new("NO PATTERN BLOCK SELECTED.").monospace().color(WARNING));
            return;
        };
        let Some(block) = song.arrangement.blocks.get(block_index) else {
            return;
        };
        if song.tracks.is_empty() {
            ui.label(RichText::new("NO TRACKS AVAILABLE.").monospace().color(WARNING));
            return;
        }

        let selected_step = self.editor_state.selected_step.unwrap_or(block_start);
        let selected_rel = selected_step.saturating_sub(block_start);
        let selected_track = self
            .editor_state
            .selected_track
            .min(song.tracks.len().saturating_sub(1));

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "PATTERN {} • LEN {} • TRACK {:02} • STEP {:02}",
                    block.pattern_name.to_uppercase(),
                    block_len,
                    selected_track + 1,
                    selected_rel + 1
                ))
                .monospace()
                .size(12.0)
                .color(ACCENT),
            );
            if let Some(step) = song
                .tracks
                .get(selected_track)
                .and_then(|track| track.steps.get(selected_step))
            {
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{} V{} G{} {}{}",
                        if step.active {
                            Self::midi_to_step_label(step.midi_note)
                        } else {
                            "REST".to_string()
                        },
                        step.velocity,
                        step.gate_percent,
                        if step.accent { "A" } else { "-" },
                        if step.fx_trigger { "F" } else { "-" },
                    ))
                    .monospace()
                    .size(12.0)
                    .color(TEXT_DIM),
                );
            }
        });

        let tracks_snapshot = song.tracks.clone();
        let mut clicked: Option<(usize, usize)> = None;
        let mut toggle_clicked = false;
        ui.add_space(4.0);
        egui::ScrollArea::both().max_height(240.0).show(ui, |ui| {
            for (track_index, track) in tracks_snapshot.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [110.0, 20.0],
                        egui::Label::new(
                            RichText::new(format!("{:02} {}", track_index + 1, track.name))
                                .monospace()
                                .size(11.0)
                                .color(if track_index == selected_track { ACCENT } else { TEXT }),
                        ),
                    );
                    for rel_step in 0..block_len {
                        let absolute_step = block_start + rel_step;
                        let step = track
                            .steps
                            .get(absolute_step)
                            .cloned()
                            .unwrap_or_else(editor::EditableStep::rest);
                        let is_selected = track_index == selected_track && rel_step == selected_rel;
                        let text = Self::step_cell_label(&step);
                        let response = ui.add(
                            egui::Button::new(
                                RichText::new(text)
                                    .monospace()
                                    .size(10.0)
                                    .color(if is_selected { BASE_BG } else { TEXT }),
                            )
                            .min_size(Vec2::new(30.0, 20.0))
                            .fill(if is_selected {
                                ACCENT
                            } else if step.active {
                                ACCENT_SOFT
                            } else {
                                PANEL_DIM_BG
                            })
                            .stroke(Stroke::new(
                                if is_selected { 2.0 } else { 1.0 },
                                if is_selected { ACCENT } else { BORDER_DIM },
                            )),
                        );
                        if response.clicked() {
                            clicked = Some((track_index, rel_step));
                        }
                        if response.double_clicked() {
                            clicked = Some((track_index, rel_step));
                            toggle_clicked = true;
                        }
                    }
                });
            }
        });
        if let Some((track_index, rel_step)) = clicked {
            self.set_pattern_cell(track_index, rel_step);
        }
        if toggle_clicked {
            self.toggle_selected_step();
        }
    }

    fn draw_arrangement_editor(&mut self, ui: &mut egui::Ui) {
        if self.editable_song.is_none() {
            ui.label(
                RichText::new("NO EDITABLE SONG. USE NEW SONG / DUPLICATE / OPEN.")
                    .monospace()
                    .color(WARNING),
            );
            return;
        }

        {
            let song = self
                .editable_song
                .as_mut()
                .expect("checked editable song presence");
            if song.arrangement.blocks.is_empty() {
                song.arrangement.blocks.push(EditableArrangementBlock {
                    pattern_name: "A".to_string(),
                    length: 16,
                });
                song.mark_dirty();
                self.editor_state.dirty = true;
            }
        }

        let mut marked_dirty = false;
        let selected_block = self.editor_state.selected_arrangement_block.unwrap_or(0);
        let selected_track = self.editor_state.selected_track;

        ui.horizontal_wrapped(|ui| {
            let song = self
                .editable_song
                .as_mut()
                .expect("editable song should still be available");
            ui.label(
                RichText::new(format!("SONG {}", song.title.to_uppercase()))
                    .monospace()
                    .strong(),
            );
            ui.separator();
            ui.label(RichText::new("TEMPO").monospace().size(12.0).color(TEXT_DIM));
            if ui
                .add(egui::DragValue::new(&mut song.tempo).range(20..=300).speed(1))
                .changed()
            {
                marked_dirty = true;
            }
            ui.separator();
            ui.label(RichText::new("SWING").monospace().size(12.0).color(TEXT_DIM));
            if ui
                .add(egui::DragValue::new(&mut song.swing).range(0..=100).speed(1))
                .changed()
            {
                marked_dirty = true;
            }
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "CURSOR {} / {}",
                    self.editor_state.selected_arrangement_block.unwrap_or(0) + 1,
                    song.arrangement.blocks.len()
                ))
                .monospace()
                .size(12.0)
                .color(ACCENT),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("TRACK {}", selected_track + 1))
                    .monospace()
                    .size(12.0)
                    .color(ACCENT),
            );
            let block_index = selected_block.min(song.arrangement.blocks.len().saturating_sub(1));
            ui.separator();
            if ui.button(RichText::new("-LEN").monospace().size(11.0)).clicked()
                && song.arrangement.blocks[block_index].length > 1
            {
                song.arrangement.blocks[block_index].length -= 1;
                let pattern_name = song.arrangement.blocks[block_index].pattern_name.clone();
                if let Some(pattern) = song
                    .patterns
                    .iter_mut()
                    .find(|pattern| pattern.name == pattern_name)
                {
                    pattern.length = song.arrangement.blocks[block_index].length;
                }
                marked_dirty = true;
            }
            if ui.button(RichText::new("+LEN").monospace().size(11.0)).clicked() {
                song.arrangement.blocks[block_index].length += 1;
                let pattern_name = song.arrangement.blocks[block_index].pattern_name.clone();
                if let Some(pattern) = song
                    .patterns
                    .iter_mut()
                    .find(|pattern| pattern.name == pattern_name)
                {
                    pattern.length = song.arrangement.blocks[block_index].length;
                }
                marked_dirty = true;
            }
        });

        if self.renaming_pattern {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("RENAME").monospace().size(12.0).color(TEXT_DIM));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.pattern_rename_buffer)
                        .font(TextStyle::Monospace)
                        .desired_width(180.0),
                );
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    self.apply_pattern_rename();
                }
                if ui.button(RichText::new("OK").monospace().size(11.0)).clicked() {
                    self.apply_pattern_rename();
                }
                if ui
                    .button(RichText::new("CANCEL").monospace().size(11.0))
                    .clicked()
                {
                    self.cancel_current_edit();
                }
            });
        }

        let (blocks, tracks, total_steps, song_dirty) = {
            let song = self
                .editable_song
                .as_ref()
                .expect("editable song should still be available");
            (
                song.arrangement.blocks.clone(),
                song.tracks.clone(),
                song.total_steps(),
                song.dirty,
            )
        };
        let selected_block = selected_block.min(blocks.len().saturating_sub(1));
        let selected_track = selected_track.min(tracks.len().saturating_sub(1));
        let mut clicked_block: Option<usize> = None;
        let mut double_clicked = false;
        let mut clicked_track: Option<usize> = None;

        ui.add_space(6.0);
        egui::ScrollArea::both().max_height(220.0).show(ui, |ui| {
            for (track_index, track) in tracks.iter().enumerate() {
                ui.horizontal(|ui| {
                    let track_selected = track_index == selected_track;
                    let track_label = format!("{:02} {}", track_index + 1, track.name.to_uppercase());
                    let track_response = ui.add(
                        egui::Button::new(
                            RichText::new(track_label)
                                .monospace()
                                .size(11.0)
                                .color(if track_selected { BASE_BG } else { TEXT }),
                        )
                        .fill(if track_selected { ACCENT } else { PANEL_DIM_BG })
                        .stroke(Stroke::new(1.0, if track_selected { ACCENT } else { BORDER_DIM }))
                        .min_size(Vec2::new(132.0, 20.0)),
                    );
                    if track_response.clicked() {
                        clicked_track = Some(track_index);
                    }

                    let mut cursor = 0usize;
                    for (block_index, block) in blocks.iter().enumerate() {
                        let end = (cursor + block.length).min(track.steps.len());
                        let activity = track
                            .steps
                            .iter()
                            .skip(cursor)
                            .take(end.saturating_sub(cursor))
                            .any(|step| step.active);
                        cursor += block.length;
                        let is_selected = block_index == selected_block;
                        let text = format!("{} {:02}", block.pattern_name, block.length);
                        let response = ui.add(
                            egui::Button::new(
                                RichText::new(text)
                                    .monospace()
                                    .size(11.0)
                                    .color(if is_selected { BASE_BG } else { TEXT }),
                            )
                            .fill(if is_selected {
                                ACCENT
                            } else if activity {
                                ACCENT_SOFT
                            } else {
                                PANEL_DIM_BG
                            })
                            .stroke(Stroke::new(1.0, if is_selected { ACCENT } else { BORDER_DIM }))
                            .min_size(Vec2::new(78.0, 20.0)),
                        );
                        if response.clicked() {
                            clicked_block = Some(block_index);
                            clicked_track = Some(track_index);
                        }
                        if response.double_clicked() {
                            clicked_block = Some(block_index);
                            clicked_track = Some(track_index);
                            double_clicked = true;
                        }
                    }
                });
            }
        });

        if let Some(track_index) = clicked_track {
            self.editor_state.selected_track = track_index;
            self.runtime.selected_track = track_index;
        }
        if let Some(block_index) = clicked_block {
            self.editor_state.selected_arrangement_block = Some(block_index);
        }
        if double_clicked {
            self.open_selected_pattern();
        }

        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "STEPS {} • BLOCKS {} • DIRTY {}",
                total_steps,
                blocks.len(),
                if self.editor_state.dirty || song_dirty { "YES" } else { "NO" }
            ))
            .monospace()
            .size(12.0)
            .color(TEXT_DIM),
        );

        if marked_dirty {
            if let Some(song) = self.editable_song.as_mut() {
                song.mark_dirty();
            }
            self.editor_state.dirty = true;
        }
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
        let render_state_label = if self.current_render().is_some() {
            "READY"
        } else {
            "IDLE"
        };
        let (playback_state, playback_color) = match self.playback.state() {
            PlaybackState::Playing => ("PLAYING", ACCENT),
            PlaybackState::Stopped => ("STOPPED", TEXT_DIM),
            PlaybackState::Error(_) => ("ERROR", WARNING),
        };
        let track_label = self
            .selected_track()
            .map(|track| track.name.to_uppercase())
            .unwrap_or_else(|| "--".to_string());

        Self::retro_panel(
            ui,
            "STATUS LINE",
            false,
            Some("TAB / SHIFT+TAB CYCLES PANELS • D/S/W/P/E/G/I/F DIRECT FOCUS"),
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
                    ui.label(
                        RichText::new(format!(
                            "DEMO {} • TRACK {} • FOCUS {}",
                            self.selected_demo().key.to_uppercase(),
                            track_label,
                            self.focus_label()
                        ))
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("MODE {}", self.editor_state.mode.label()))
                            .monospace()
                            .size(12.0)
                            .color(match self.editor_state.mode {
                                EditorMode::Browser => TEXT_DIM,
                                EditorMode::Edit => ACCENT,
                                EditorMode::Preview => WARNING,
                            }),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("RENDER {render_state_label}"))
                            .monospace()
                            .size(12.0)
                            .color(if render_state_label == "READY" {
                                ACCENT
                            } else {
                                TEXT_DIM
                            }),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("PLAYBACK {playback_state}"))
                            .monospace()
                            .size(12.0)
                            .color(playback_color),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "LAST ERROR {}",
                            self.runtime.last_error.as_deref().unwrap_or("NONE")
                        ))
                        .monospace()
                        .size(12.0)
                        .color(if self.runtime.last_error.is_some() {
                            WARNING
                        } else {
                            TEXT_DIM
                        }),
                    );
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for hint in [
                        "[UP/DOWN] DEMO OR TRACK",
                        "[A] ARRANGEMENT FOCUS",
                        "[E/G] PATTERN EDITOR FOCUS",
                        "[ENTER] RENDER",
                        "[SPACE] PLAY / STOP",
                        "[ESC] STOP",
                        "[N/D/DEL/R] ARRANGE EDIT",
                        "[ARROWS/ENTER] STEP EDIT",
                        "[+/-] OCTAVE • [A/F] ACCENT+FX • [G/V] GATE+VEL",
                        "[C/X/P] COPY/CUT/PASTE STEP",
                        "[CTRL+S] SAVE • [CTRL+SHIFT+S] SAVE AS",
                        "[CTRL+R] PREVIEW RENDER",
                        "[TAB] NEXT PANEL",
                        "[D/S/W/P/E/G/I/F] DIRECT FOCUS",
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

    fn midi_to_step_label(midi: u8) -> String {
        const NOTES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        if midi == 0 {
            return "REST".to_string();
        }
        let semitone = (midi % 12) as usize;
        let octave = midi as i32 / 12 - 1;
        format!("{}{}", NOTES[semitone], octave)
    }

    fn step_cell_label(step: &editor::EditableStep) -> String {
        if !step.active || step.midi_note == 0 {
            return "··".to_string();
        }
        let mut label = Self::midi_to_step_label(step.midi_note);
        if label.len() > 3 {
            label.truncate(3);
        }
        if step.accent {
            label.push('!');
        }
        if step.fx_trigger {
            label.push('*');
        }
        label
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
            ctx.request_repaint_after(Duration::from_millis(60));
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
                        RichText::new(if self.editor_state.mode == EditorMode::Browser {
                            "READ-ONLY"
                        } else {
                            "EDITABLE"
                        })
                            .monospace()
                            .size(12.0)
                            .color(if self.editor_state.mode == EditorMode::Browser {
                                BORDER
                            } else {
                                ACCENT
                            }),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(RichText::new("NEW SONG").monospace().size(12.0))
                        .clicked()
                    {
                        self.create_new_song();
                    }
                    if ui
                        .button(
                            RichText::new("DUPLICATE DEMO AS EDITABLE")
                                .monospace()
                                .size(12.0),
                        )
                        .clicked()
                    {
                        self.duplicate_demo_as_editable();
                    }
                    if ui
                        .button(RichText::new("OPEN EDITABLE SONG").monospace().size(12.0))
                        .clicked()
                    {
                        self.open_editable_song_from_path();
                    }
                    if self.editor_state.mode != EditorMode::Browser
                        && ui
                            .button(RichText::new("BROWSER MODE").monospace().size(12.0))
                            .clicked()
                    {
                        self.set_mode(EditorMode::Browser);
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.editor_open_path)
                            .hint_text("PATH TO .ABC")
                            .font(TextStyle::Monospace),
                    );
                });
            });

        egui::TopBottomPanel::bottom("runtime_footer")
            .resizable(false)
            .show(ctx, |ui| self.draw_status_line(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.columns(2, |columns| {
                    self.draw_demo_browser(&mut columns[0]);
                    self.draw_stats_panel(&mut columns[1]);
                });
                ui.add_space(8.0);
                self.draw_waveform_panel(ui);
                ui.add_space(8.0);
                self.draw_pattern_panel(ui);
                ui.add_space(8.0);
                self.draw_pattern_editor_panel(ui);
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
                Some("pattern-editor") | Some("editor") => Some(FocusArea::PatternEditor),
                Some("instrument") | Some("inspector") => Some(FocusArea::InstrumentInspector),
                Some("fx") => Some(FocusArea::FxInspector),
                _ => None,
            },
            editable_source: env::var("MEMDECK_GUI_BOOT_EDITABLE").ok().and_then(|value| {
                let normalized = value.to_lowercase();
                if normalized == "new" || normalized == "duplicate" {
                    Some(normalized)
                } else {
                    None
                }
            }),
            mode: match env::var("MEMDECK_GUI_BOOT_MODE").ok().as_deref() {
                Some("browser") => Some(EditorMode::Browser),
                Some("edit") => Some(EditorMode::Edit),
                Some("preview") => Some(EditorMode::Preview),
                _ => None,
            },
            apply_pattern_edits: env_flag("MEMDECK_GUI_BOOT_PATTERN_EDITS"),
        }
    }
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn selected_track_index_stays_valid() {
        let mut app = MemDeckGuiApp::default();
        let demo_index = app
            .demos
            .iter()
            .position(|demo| {
                demo.overview
                    .as_ref()
                    .is_some_and(|overview| !overview.tracks.is_empty())
            })
            .expect("expected at least one demo with tracks");
        app.select_demo(demo_index);

        app.runtime.selected_track = usize::MAX;
        app.sync_selected_track();

        let track_count = app
            .selected_overview()
            .map(|overview| overview.tracks.len())
            .unwrap_or(0);
        assert!(
            track_count > 0,
            "selected demo should have at least one track"
        );
        assert!(
            app.runtime.selected_track < track_count,
            "selected track must stay in range"
        );
    }

    #[test]
    fn selecting_new_demo_resets_track_and_render_state() {
        let mut app = MemDeckGuiApp::default();
        let valid_demos = app
            .demos
            .iter()
            .enumerate()
            .filter_map(|(index, demo)| demo.overview.as_ref().map(|_| index))
            .take(2)
            .collect::<Vec<_>>();
        assert!(
            valid_demos.len() >= 2,
            "expected at least two valid showcase demos"
        );

        app.select_demo(valid_demos[0]);
        app.runtime.selected_track = 2;
        app.runtime.rendered_audio = Some(RenderState {
            demo_key: app.demos[valid_demos[0]].key.clone(),
            samples: Arc::<[u8]>::from(vec![0_u8, 1, 2, 3]),
            stats: None,
        });

        app.select_demo(valid_demos[1]);

        assert_eq!(
            app.runtime.selected_track, 0,
            "demo switch should reset track"
        );
        assert!(
            app.runtime.rendered_audio.is_none(),
            "demo switch should clear stale render state"
        );
    }

    #[test]
    fn tab_focus_cycle_is_deterministic() {
        let mut app = MemDeckGuiApp::default();
        app.runtime.focus = FocusArea::DemoBrowser;

        let expected_forward = [
            FocusArea::RenderStats,
            FocusArea::WaveformView,
            FocusArea::PatternOverview,
            FocusArea::PatternEditor,
            FocusArea::InstrumentInspector,
            FocusArea::FxInspector,
            FocusArea::DemoBrowser,
        ];
        for expected in expected_forward {
            app.handle_key_press(egui::Key::Tab, false);
            assert_eq!(app.runtime.focus, expected);
        }

        app.handle_key_press(egui::Key::Tab, true);
        assert_eq!(app.runtime.focus, FocusArea::FxInspector);
    }

    #[test]
    fn direct_focus_shortcuts_target_expected_panel() {
        let mut app = MemDeckGuiApp::default();
        let checks = [
            (egui::Key::D, FocusArea::DemoBrowser),
            (egui::Key::S, FocusArea::RenderStats),
            (egui::Key::W, FocusArea::WaveformView),
            (egui::Key::P, FocusArea::PatternOverview),
            (egui::Key::E, FocusArea::PatternEditor),
            (egui::Key::G, FocusArea::PatternEditor),
            (egui::Key::I, FocusArea::InstrumentInspector),
            (egui::Key::F, FocusArea::FxInspector),
        ];

        for (key, expected_focus) in checks {
            app.runtime.focus = FocusArea::DemoBrowser;
            app.handle_key_press(key, false);
            assert_eq!(app.runtime.focus, expected_focus);
        }
    }

    #[test]
    fn arrow_navigation_routes_between_demos_and_tracks() {
        let mut app = MemDeckGuiApp::default();
        let demo_count = app.demos.len();
        assert!(demo_count >= 2, "expected at least two demos for navigation");

        app.runtime.focus = FocusArea::DemoBrowser;
        app.runtime.selected_demo = 0;
        app.handle_key_press(egui::Key::ArrowDown, false);
        assert_eq!(
            app.runtime.selected_demo, 1,
            "down in demo browser should move to next demo"
        );
        app.handle_key_press(egui::Key::ArrowUp, false);
        assert_eq!(
            app.runtime.selected_demo, 0,
            "up in demo browser should move to previous demo"
        );

        let demo_index = app
            .demos
            .iter()
            .position(|demo| {
                demo.overview
                    .as_ref()
                    .is_some_and(|overview| overview.tracks.len() > 1)
            })
            .expect("expected at least one demo with multiple tracks");
        app.select_demo(demo_index);
        app.runtime.focus = FocusArea::PatternOverview;
        app.runtime.selected_track = 0;
        app.handle_key_press(egui::Key::ArrowDown, false);
        assert_eq!(
            app.runtime.selected_track, 1,
            "down outside demo browser should move selected track"
        );
        app.handle_key_press(egui::Key::ArrowUp, false);
        assert_eq!(
            app.runtime.selected_track, 0,
            "up outside demo browser should move selected track"
        );
    }

    #[test]
    fn render_preview_from_editable_song_sets_preview_mode() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        app.runtime.focus = FocusArea::PatternEditor;
        app.toggle_selected_step();
        app.render_editable_preview();
        assert_eq!(app.editor_state.mode, EditorMode::Preview);
        assert!(
            app.runtime
                .rendered_audio
                .as_ref()
                .is_some_and(|state| state.demo_key == "__editable__"),
            "preview render should use editable render key"
        );
    }
}
