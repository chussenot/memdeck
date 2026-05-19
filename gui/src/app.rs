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
const RECENT_SONGS_LIMIT: usize = 8;

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

#[derive(Clone, Debug)]
struct RecentSongEntry {
    path: PathBuf,
    source_label: String,
}

#[derive(Clone, Debug)]
enum DeferredAction {
    NewSong,
    DuplicateFromBrowserDemo,
    DuplicateCurrentEditable,
    CloseSong,
    SwitchMode(EditorMode),
    OpenSongDialog,
    OpenSongPath(PathBuf),
    QuitApplication,
}

#[derive(Clone, Debug)]
enum ActiveDialog {
    OpenSongPath,
    SaveAsPath,
    UnsavedChanges { action: DeferredAction },
}

#[derive(Default)]
struct BootOptions {
    demo_key: Option<String>,
    auto_render: bool,
    focus: Option<FocusArea>,
    editable_source: Option<String>,
    mode: Option<EditorMode>,
    apply_pattern_edits: bool,
    dialog: Option<String>,
}

struct RuntimeState {
    selected_demo: usize,
    selected_track: usize,
    rendered_audio: Option<RenderState>,
    focus: FocusArea,
    last_error: Option<String>,
    /// User-managed playhead position in [0,1] across the current render.
    /// When playback is active, this is overridden by live progress.
    /// When playback is stopped, this is where `Space` will resume from.
    playhead_position: f32,
    /// Tracks an in-flight drag on the waveform so the visible playhead can
    /// preview the seek target before the audio respawns on release.
    waveform_drag_preview: Option<f32>,
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
    user_song_root: PathBuf,
    recent_songs: Vec<RecentSongEntry>,
    active_dialog: Option<ActiveDialog>,
    request_quit: bool,
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
                playhead_position: 0.0,
                waveform_drag_preview: None,
            },
            status,
            boot_options,
            boot_applied: false,
            editor_state: EditorState::default(),
            editable_song: None,
            editor_open_path: String::new(),
            user_song_root: Self::default_user_song_root(),
            recent_songs: Vec::new(),
            active_dialog: None,
            request_quit: false,
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

    fn default_user_song_root() -> PathBuf {
        if let Some(path) = env::var_os("MEMDECK_USER_SONG_DIR") {
            return PathBuf::from(path);
        }
        if let Some(path) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(path).join("memdeck").join("music").join("user");
        }
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("memdeck")
                .join("music")
                .join("user");
        }
        PathBuf::from("data").join("music").join("user")
    }

    fn ensure_user_song_root(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.user_song_root)
            .map_err(|error| format!("FAILED TO PREPARE USER SONG FOLDER • {error}"))
    }

    fn sanitize_song_file_stem(title: &str) -> String {
        let mut normalized = title
            .trim()
            .to_lowercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>();
        while normalized.contains("__") {
            normalized = normalized.replace("__", "_");
        }
        let trimmed = normalized.trim_matches('_');
        if trimmed.is_empty() {
            "untitled".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn suggest_user_song_path(&self, title: &str) -> PathBuf {
        let stem = Self::sanitize_song_file_stem(title);
        let mut index = 1usize;
        loop {
            let file_name = if index == 1 {
                format!("{stem}.abc")
            } else {
                format!("{stem}_{index:02}.abc")
            };
            let candidate = self.user_song_root.join(file_name);
            if !candidate.exists() {
                return candidate;
            }
            index += 1;
        }
    }

    fn effective_song_dirty(&self) -> bool {
        self.editor_state.dirty || self.editable_song.as_ref().is_some_and(|song| song.dirty)
    }

    fn is_bundled_demo_path(&self, path: &std::path::Path) -> bool {
        self.demos.iter().any(|demo| demo.path == path)
    }

    fn remember_recent_song(&mut self, path: PathBuf, source_label: String) {
        self.recent_songs.retain(|entry| entry.path != path);
        self.recent_songs
            .insert(0, RecentSongEntry { path, source_label });
        self.recent_songs.truncate(RECENT_SONGS_LIMIT);
    }

    fn selected_demo(&self) -> &DemoEntry {
        &self.demos[self.runtime.selected_demo]
    }

    fn selected_overview(&self) -> Option<&DemoOverview> {
        self.selected_demo().overview.as_ref()
    }

    fn selected_browser_track(&self) -> Option<&TrackOverview> {
        let overview = self.selected_overview()?;
        overview.tracks.get(self.clamped_track_index(overview))
    }

    fn selected_browser_fx_bus(&self) -> Option<&FxBusOverview> {
        let overview = self.selected_overview()?;
        let track = self.selected_browser_track()?;
        overview.fx_buses.get(track.fx_bus)
    }

    fn active_track_index(&self) -> usize {
        if self.editor_state.mode == EditorMode::Browser {
            self.runtime.selected_track
        } else {
            self.editor_state.selected_track
        }
    }

    fn set_active_track_index(&mut self, track_index: usize) {
        self.runtime.selected_track = track_index;
        self.editor_state.selected_track = track_index;
    }

    fn selected_editable_track(&self) -> Option<&editor::EditableTrack> {
        let song = self.editable_song.as_ref()?;
        song.tracks.get(self.active_track_index())
    }

    fn selected_editable_instrument_index(&self) -> Option<usize> {
        let song = self.editable_song.as_ref()?;
        let track = self.selected_editable_track()?;
        song.instruments
            .iter()
            .position(|instrument| instrument.name == track.instrument_ref)
            .or_else(|| (!song.instruments.is_empty()).then_some(0))
    }

    fn selected_editable_fx_bus_index(&self) -> Option<usize> {
        let song = self.editable_song.as_ref()?;
        let instrument_index = self.selected_editable_instrument_index()?;
        song.instruments
            .get(instrument_index)
            .map(|instrument| instrument.fx_bus)
    }

    fn current_render(&self) -> Option<&RenderState> {
        let render = self.runtime.rendered_audio.as_ref()?;
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
        if let Some(preview) = self.runtime.waveform_drag_preview {
            return Some(preview.clamp(0.0, 1.0));
        }
        if self.playback.is_playing() {
            self.playback.progress()
        } else if self.current_render().is_some() {
            Some(self.runtime.playhead_position.clamp(0.0, 1.0))
        } else {
            None
        }
    }

    fn current_render_duration_secs(&self) -> Option<f32> {
        let stats = self.current_render().and_then(|render| render.stats)?;
        let secs = stats.duration_ms as f32 / 1000.0;
        if secs > 0.0 {
            Some(secs)
        } else {
            None
        }
    }

    /// Move the user-managed playhead to `position` (clamped to [0,1)).
    /// If audio is currently playing, restart it from the new offset so the
    /// audible and visible playheads stay in sync.
    fn seek_playhead(&mut self, position: f32) {
        if self.current_render().is_none() {
            return;
        }
        let target = position.clamp(0.0, 0.999);
        self.runtime.playhead_position = target;
        self.runtime.waveform_drag_preview = None;

        if !self.playback.is_playing() {
            return;
        }
        let Some(samples) = self.current_render().map(|render| render.samples.clone()) else {
            return;
        };
        match self.playback.start_pcm_at(samples.as_ref(), target) {
            Ok(()) => {
                let demo_name = self.selected_demo().key.to_uppercase();
                self.set_status(
                    StatusTone::Active,
                    format!("PLAYING • {demo_name} @ {}", Self::format_position(target, self.current_render_duration_secs())),
                );
            }
            Err(error) => self.set_status(
                StatusTone::Warning,
                format!("SEEK ERROR • {error}"),
            ),
        }
    }

    fn nudge_playhead_seconds(&mut self, delta_seconds: f32) {
        let Some(duration) = self.current_render_duration_secs() else {
            return;
        };
        let current_secs = self.runtime.playhead_position * duration;
        let target_secs = (current_secs + delta_seconds).max(0.0);
        self.seek_playhead((target_secs / duration).min(0.999));
    }

    fn format_position(position: f32, duration_secs: Option<f32>) -> String {
        let total = duration_secs.unwrap_or(0.0);
        let at = position.clamp(0.0, 1.0) * total;
        format!("{} / {}", Self::format_seconds(at), Self::format_seconds(total))
    }

    fn format_seconds(secs: f32) -> String {
        let total = secs.max(0.0) as u32;
        let m = total / 60;
        let s = total % 60;
        format!("{m}:{s:02}")
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

    fn sync_track_selection_state(&mut self) {
        if self.editor_state.mode == EditorMode::Browser {
            if let Some(overview) = self.selected_overview() {
                self.runtime.selected_track = self.clamped_track_index(overview);
            } else {
                self.runtime.selected_track = 0;
            }
            return;
        }

        let Some(song) = self.editable_song.as_ref() else {
            self.editor_state.selected_track = 0;
            self.runtime.selected_track = 0;
            return;
        };
        if song.tracks.is_empty() {
            self.editor_state.selected_track = 0;
            self.runtime.selected_track = 0;
            return;
        }
        let clamped = self
            .editor_state
            .selected_track
            .min(song.tracks.len().saturating_sub(1));
        self.set_active_track_index(clamped);
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

        if let Some(dialog) = self.boot_options.dialog.as_deref() {
            match dialog {
                "open" => self.active_dialog = Some(ActiveDialog::OpenSongPath),
                "save-as" => self.active_dialog = Some(ActiveDialog::SaveAsPath),
                _ => {}
            }
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
        self.runtime.playhead_position = 0.0;
        self.runtime.waveform_drag_preview = None;

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
        if self.editor_state.mode == EditorMode::Browser {
            let Some((track_count, next, label)) = self.selected_overview().map(|overview| {
                let track_count = overview.tracks.len();
                if track_count == 0 {
                    return (0, 0, None);
                }

                let next = (self.clamped_track_index(overview) as isize + delta)
                    .clamp(0, track_count.saturating_sub(1) as isize)
                    as usize;
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
            return;
        }

        let Some((track_count, next)) = self.editable_song.as_ref().map(|song| {
            let track_count = song.tracks.len();
            if track_count == 0 {
                return (0, 0);
            }
            let next = (self.active_track_index() as isize + delta)
                .clamp(0, track_count.saturating_sub(1) as isize) as usize;
            (track_count, next)
        }) else {
            return;
        };
        if track_count == 0 {
            return;
        }
        self.set_active_track_index(next);
        self.normalize_pattern_cursor();
        if let Some(track) = self.selected_editable_track() {
            self.set_status(
                StatusTone::Normal,
                format!(
                    "TRACK {:02} • {} • BUS {}",
                    next + 1,
                    track.name.to_uppercase(),
                    self.selected_editable_fx_bus_index().unwrap_or(0)
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

    fn open_unsaved_changes_dialog(&mut self, action: DeferredAction) {
        self.active_dialog = Some(ActiveDialog::UnsavedChanges { action });
    }

    fn request_action(&mut self, action: DeferredAction) {
        if self.editor_state.mode != EditorMode::Browser && self.effective_song_dirty() {
            self.open_unsaved_changes_dialog(action);
            return;
        }
        self.execute_action(action);
    }

    fn execute_action(&mut self, action: DeferredAction) {
        match action {
            DeferredAction::NewSong => self.create_new_song(),
            DeferredAction::DuplicateFromBrowserDemo => self.duplicate_demo_as_editable(),
            DeferredAction::DuplicateCurrentEditable => self.duplicate_current_editable_song(),
            DeferredAction::CloseSong => self.close_current_song(),
            DeferredAction::SwitchMode(mode) => self.set_mode(mode),
            DeferredAction::OpenSongDialog => {
                if self.editor_open_path.trim().is_empty() {
                    self.editor_open_path = self.user_song_root.to_string_lossy().to_string();
                }
                self.active_dialog = Some(ActiveDialog::OpenSongPath);
            }
            DeferredAction::OpenSongPath(path) => self.open_editable_song_from_path(path),
            DeferredAction::QuitApplication => {
                self.request_quit = true;
            }
        }
    }

    fn resolve_unsaved_action(&mut self, save_before: bool, discard: bool) {
        let Some(ActiveDialog::UnsavedChanges { action }) = self.active_dialog.clone() else {
            return;
        };
        if save_before && !self.save_editable_song(false) {
            return;
        }
        if discard || save_before {
            self.active_dialog = None;
            self.execute_action(action);
        }
    }

    fn set_mode(&mut self, mode: EditorMode) {
        if self.editor_state.mode == mode {
            return;
        }
        if matches!(mode, EditorMode::Browser) {
            self.stop_playback(false);
        }
        if self.editable_song.is_none() && mode != EditorMode::Browser {
            self.set_status(StatusTone::Warning, "MODE SWITCH FAILED • NO EDITABLE SONG");
            return;
        }
        if self.editor_state.mode == EditorMode::Preview && mode == EditorMode::Edit {
            self.stop_playback(false);
        }
        self.editor_state.mode = mode;
        if mode != EditorMode::Browser {
            self.sync_arrangement_selection_with_pattern(true);
        }
        self.sync_track_selection_state();
    }

    fn sync_arrangement_selection_with_pattern(&mut self, preserve_step_offset: bool) {
        let Some(song) = self.editable_song.as_ref() else {
            self.editor_state.selected_arrangement_block = None;
            self.editor_state.selected_pattern = None;
            self.editor_state.selected_step = None;
            return;
        };

        if song.arrangement.blocks.is_empty() {
            self.editor_state.selected_arrangement_block = None;
            self.editor_state.selected_pattern = None;
            self.editor_state.selected_step = None;
            return;
        }

        let block_index = self
            .editor_state
            .selected_arrangement_block
            .unwrap_or(0)
            .min(song.arrangement.blocks.len().saturating_sub(1));
        self.editor_state.selected_arrangement_block = Some(block_index);

        let block = &song.arrangement.blocks[block_index];
        self.editor_state.selected_pattern = song
            .patterns
            .iter()
            .position(|pattern| pattern.name == block.pattern_name);

        let block_start = song
            .arrangement
            .blocks
            .iter()
            .take(block_index)
            .map(|item| item.length)
            .sum::<usize>();
        let block_len = block.length.max(1);
        let next_step = if preserve_step_offset {
            let relative = self
                .editor_state
                .selected_step
                .unwrap_or(block_start)
                .saturating_sub(block_start)
                .min(block_len.saturating_sub(1));
            block_start + relative
        } else {
            block_start
        };
        self.editor_state.selected_step = Some(next_step);
    }

    fn invalidate_editable_preview(&mut self) {
        if self
            .runtime
            .rendered_audio
            .as_ref()
            .is_some_and(|render| render.demo_key == "__editable__")
        {
            self.runtime.rendered_audio = None;
        }
        if self.editor_state.mode == EditorMode::Preview {
            self.editor_state.mode = EditorMode::Edit;
        }
    }

    fn create_new_song(&mut self) {
        self.stop_playback(false);
        let song = EditableSong::new_song();
        self.editor_open_path = self
            .suggest_user_song_path(&song.title)
            .to_string_lossy()
            .to_string();
        self.editable_song = Some(song);
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
        self.set_active_track_index(0);
        self.sync_arrangement_selection_with_pattern(false);
        self.renaming_pattern = false;
        self.pattern_rename_buffer.clear();
        self.set_status(StatusTone::Active, "EDIT MODE • NEW SONG");
    }

    fn duplicate_demo_as_editable(&mut self) {
        let demo_key = self.selected_demo().key.clone();
        let path = self.selected_demo().path.clone();
        match editor::load_editable_song_from_path(&path) {
            Ok(mut song) => {
                song.source_path = None;
                song.dirty = true;
                self.editor_open_path = self
                    .suggest_user_song_path(&song.title)
                    .to_string_lossy()
                    .to_string();
                self.remember_recent_song(
                    PathBuf::from(&self.editor_open_path),
                    format!("DUPLICATED DEMO • {}", demo_key.to_uppercase()),
                );
                self.stop_playback(false);
                self.editable_song = Some(song);
                self.editor_state.mode = EditorMode::Edit;
                self.editor_state.selected_arrangement_block = Some(0);
                self.editor_state.selected_pattern = Some(0);
                self.editor_state.selected_step = Some(0);
                self.set_active_track_index(0);
                self.editor_state.dirty = true;
                self.editor_state.last_saved_path = None;
                self.editor_state.last_error = None;
                self.sync_arrangement_selection_with_pattern(false);
                self.set_status(StatusTone::Active, "EDIT MODE • DEMO DUPLICATED");
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("DUPLICATE FAILED • {error}"));
            }
        }
    }

    fn duplicate_current_editable_song(&mut self) {
        let Some(song) = self.editable_song.clone() else {
            self.set_status(StatusTone::Warning, "DUPLICATE FAILED • NO EDITABLE SONG");
            return;
        };
        let mut duplicated = song;
        duplicated.source_path = None;
        duplicated.dirty = true;
        self.editor_open_path = self
            .suggest_user_song_path(&duplicated.title)
            .to_string_lossy()
            .to_string();
        self.stop_playback(false);
        self.editable_song = Some(duplicated);
        self.editor_state.mode = EditorMode::Edit;
        self.editor_state.selected_arrangement_block = Some(0);
        self.editor_state.selected_pattern = Some(0);
        self.editor_state.selected_step = Some(0);
        self.set_active_track_index(0);
        self.editor_state.dirty = true;
        self.editor_state.last_saved_path = None;
        self.editor_state.last_error = None;
        self.sync_arrangement_selection_with_pattern(false);
        self.remember_recent_song(
            PathBuf::from(&self.editor_open_path),
            "DUPLICATED EDITABLE SONG".to_string(),
        );
        self.set_status(StatusTone::Active, "EDIT MODE • SONG DUPLICATED");
    }

    fn open_editable_song_from_path(&mut self, path: PathBuf) {
        if path.as_os_str().is_empty() {
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
                self.set_active_track_index(0);
                self.editor_state.dirty = false;
                self.editor_state.last_saved_path = Some(path.clone());
                self.editor_state.last_error = None;
                self.editor_open_path = path.to_string_lossy().to_string();
                self.remember_recent_song(path.clone(), "OPENED EDITABLE SONG".to_string());
                self.sync_arrangement_selection_with_pattern(false);
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

    fn save_editable_song(&mut self, save_as: bool) -> bool {
        let Some(song_view) = self.editable_song.as_ref() else {
            self.set_status(StatusTone::Warning, "SAVE FAILED • NO EDITABLE SONG");
            return false;
        };
        let song_title = song_view.title.clone();
        let song_source_path = song_view.source_path.clone();

        let target_path = if save_as {
            let input = self.editor_open_path.trim();
            if input.is_empty() {
                self.set_status(StatusTone::Warning, "SAVE AS FAILED • ENTER A FILE PATH");
                return false;
            }
            PathBuf::from(input)
        } else {
            let requested = song_source_path
                .clone()
                .or_else(|| self.editor_state.last_saved_path.clone())
                .unwrap_or_else(|| PathBuf::from(self.editor_open_path.trim()));
            if requested.as_os_str().is_empty() || self.is_bundled_demo_path(&requested) {
                self.suggest_user_song_path(&song_title)
            } else {
                requested
            }
        };

        if target_path.as_os_str().is_empty() {
            self.set_status(StatusTone::Warning, "SAVE FAILED • NO TARGET FILE");
            return false;
        }

        if !save_as && self.is_bundled_demo_path(&target_path) {
            self.set_status(
                StatusTone::Warning,
                "SAVE BLOCKED • BUNDLED DEMO PATH REQUIRES EXPLICIT SAVE AS",
            );
            return false;
        }

        if let Err(error) = self.ensure_user_song_root() {
            self.editor_state.set_error(error.clone());
            self.set_status(StatusTone::Warning, error);
            return false;
        }

        let Some(song) = self.editable_song.as_mut() else {
            self.set_status(StatusTone::Warning, "SAVE FAILED • NO EDITABLE SONG");
            return false;
        };

        match editor::save_editable_song_to_path(song, &target_path) {
            Ok(()) => {
                self.editor_state.dirty = false;
                self.editor_state.last_saved_path = Some(target_path.clone());
                self.editor_state.clear_error();
                self.editor_open_path = target_path.to_string_lossy().to_string();
                self.remember_recent_song(target_path.clone(), "SAVED EDITABLE SONG".to_string());
                self.set_status(
                    StatusTone::Active,
                    format!("SAVED • {}", target_path.display()),
                );
                true
            }
            Err(error) => {
                self.editor_state.set_error(error.clone());
                self.set_status(StatusTone::Warning, format!("SAVE FAILED • {error}"));
                false
            }
        }
    }

    fn close_current_song(&mut self) {
        self.stop_playback(false);
        self.invalidate_editable_preview();
        self.editable_song = None;
        self.editor_state.mode = EditorMode::Browser;
        self.editor_state.selected_pattern = None;
        self.editor_state.selected_arrangement_block = None;
        self.editor_state.selected_step = None;
        self.editor_state.dirty = false;
        self.editor_state.last_saved_path = None;
        self.editor_state.last_error = None;
        self.editor_open_path.clear();
        self.set_status(StatusTone::Normal, "BROWSER MODE • SONG CLOSED");
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

        let tmp_path =
            std::env::temp_dir().join(format!("memdeck-edit-preview-{}.abc", std::process::id()));
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
                self.runtime.playhead_position = 0.0;
                self.runtime.waveform_drag_preview = None;
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
        self.sync_arrangement_selection_with_pattern(true);
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
        self.sync_arrangement_selection_with_pattern(false);
        self.invalidate_editable_preview();
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
            self.invalidate_editable_preview();
        }
        self.sync_arrangement_selection_with_pattern(false);
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
            self.invalidate_editable_preview();
        }
        self.sync_arrangement_selection_with_pattern(false);
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
        self.sync_arrangement_selection_with_pattern(false);
        self.invalidate_editable_preview();
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
        self.sync_arrangement_selection_with_pattern(true);
        self.invalidate_editable_preview();
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
        if self.editor_state.mode != EditorMode::Browser
            && self.runtime.focus == FocusArea::PatternEditor
        {
            self.focus_panel(FocusArea::PatternOverview);
            return;
        }
        self.stop_playback(true);
    }

    fn open_selected_pattern(&mut self) {
        let Some(block_name) = self.editable_song.as_ref().and_then(|song| {
            let block_index = self.editor_state.selected_arrangement_block?;
            song.arrangement
                .blocks
                .get(block_index)
                .map(|block| block.pattern_name.clone())
        }) else {
            return;
        };
        self.sync_arrangement_selection_with_pattern(false);
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
        let next_track =
            (self.editor_state.selected_track as isize + delta_track).clamp(0, track_max);
        self.set_active_track_index(next_track as usize);

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
        self.set_active_track_index(track_index.min(song.tracks.len().saturating_sub(1)));
        self.editor_state.selected_step =
            Some(block_start + relative_step.min(block_len.saturating_sub(1)));
    }

    fn mark_editor_dirty(&mut self) {
        if let Some(song) = self.editable_song.as_mut() {
            song.mark_dirty();
            self.editor_state.dirty = song.dirty;
        } else {
            self.editor_state.dirty = true;
        }
        self.invalidate_editable_preview();
    }

    fn with_selected_step_mut(&mut self, mutate: impl FnOnce(&mut editor::EditableStep)) -> bool {
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
                self.invalidate_editable_preview();
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
        self.set_active_track_index(0);
        self.editor_state.selected_step = Some(0);
        self.sync_arrangement_selection_with_pattern(false);
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
                self.runtime.playhead_position = 0.0;
                self.runtime.waveform_drag_preview = None;
                self.sync_track_selection_state();
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

        // If the user left the playhead at (or past) the end on a previous
        // finished run, restart from the beginning — Space-on-finished is a
        // far more useful default than spawning a zero-length tail.
        if self.runtime.playhead_position >= 0.999 {
            self.runtime.playhead_position = 0.0;
        }
        let start_at = self.runtime.playhead_position;
        let duration = self.current_render_duration_secs();

        match self.playback.start_pcm_at(samples.as_ref(), start_at) {
            Ok(()) => {
                let position_label = Self::format_position(start_at, duration);
                self.set_status(
                    StatusTone::Active,
                    format!("PLAYING • {demo_name} @ {position_label}"),
                );
            }
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
            // Park the playhead at the end so users see where playback ran out;
            // toggle_playback rewinds to 0 the next time Space is pressed.
            self.runtime.playhead_position = 1.0;
            self.runtime.waveform_drag_preview = None;
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
            if let Some(dialog) = self.active_dialog.clone() {
                if input.key_pressed(egui::Key::Escape) {
                    self.active_dialog = None;
                    return;
                }
                if let ActiveDialog::UnsavedChanges { .. } = dialog {
                    if input.key_pressed(egui::Key::S) {
                        self.resolve_unsaved_action(true, false);
                        return;
                    }
                    if input.key_pressed(egui::Key::D) {
                        self.resolve_unsaved_action(false, true);
                        return;
                    }
                }
                return;
            }

            if input.modifiers.command && input.key_pressed(egui::Key::N) {
                self.request_action(DeferredAction::NewSong);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::O) {
                self.request_action(DeferredAction::OpenSongDialog);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::D) {
                if self.editor_state.mode == EditorMode::Browser {
                    self.request_action(DeferredAction::DuplicateFromBrowserDemo);
                } else {
                    self.request_action(DeferredAction::DuplicateCurrentEditable);
                }
            }
            if input.modifiers.command && input.modifiers.shift && input.key_pressed(egui::Key::S) {
                if self.editor_state.mode != EditorMode::Browser {
                    self.active_dialog = Some(ActiveDialog::SaveAsPath);
                }
            }
            if input.modifiers.command && input.key_pressed(egui::Key::S) && !input.modifiers.shift
            {
                if self.editor_state.mode != EditorMode::Browser {
                    let _ = self.save_editable_song(false);
                }
            }
            if input.modifiers.command && input.key_pressed(egui::Key::R) {
                if self.editor_state.mode == EditorMode::Browser {
                    let _ = self.render_selected_demo();
                } else {
                    self.render_editable_preview();
                }
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
                egui::Key::O,
                egui::Key::V,
                egui::Key::W,
                egui::Key::X,
                egui::Key::P,
                egui::Key::I,
                egui::Key::F,
                egui::Key::Home,
                egui::Key::End,
            ] {
                if input.key_pressed(key) {
                    self.handle_key_press_with_modifiers(
                        key,
                        input.modifiers.shift,
                        input.modifiers.command,
                    );
                }
            }
        });
    }

    fn handle_key_press(&mut self, key: egui::Key, shift: bool) {
        self.handle_key_press_with_modifiers(key, shift, false);
    }

    fn handle_key_press_with_modifiers(&mut self, key: egui::Key, shift: bool, command: bool) {
        match key {
            egui::Key::Tab => self.cycle_focus(shift),
            egui::Key::ArrowLeft => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.move_pattern_cursor(0, -1);
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    if command {
                        self.arrangement_reorder_selected(-1);
                    } else {
                        self.arrangement_move_cursor(-1);
                    }
                } else if self.runtime.focus == FocusArea::WaveformView {
                    self.nudge_playhead_seconds(if shift { -5.0 } else { -1.0 });
                }
            }
            egui::Key::ArrowRight => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.move_pattern_cursor(0, 1);
                } else if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternOverview
                {
                    if command {
                        self.arrangement_reorder_selected(1);
                    } else {
                        self.arrangement_move_cursor(1);
                    }
                } else if self.runtime.focus == FocusArea::WaveformView {
                    self.nudge_playhead_seconds(if shift { 5.0 } else { 1.0 });
                }
            }
            egui::Key::ArrowUp => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.move_pattern_cursor(-1, 0);
                } else if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(-1);
                } else {
                    self.move_track_selection(-1);
                }
            }
            egui::Key::ArrowDown => {
                if self.editor_state.mode != EditorMode::Browser
                    && self.runtime.focus == FocusArea::PatternEditor
                {
                    self.move_pattern_cursor(1, 0);
                } else if self.runtime.focus == FocusArea::DemoBrowser {
                    self.move_demo_selection(1);
                } else {
                    self.move_track_selection(1);
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
            egui::Key::Home => {
                if self.runtime.focus == FocusArea::WaveformView {
                    self.seek_playhead(0.0);
                }
            }
            egui::Key::End => {
                if self.runtime.focus == FocusArea::WaveformView {
                    self.seek_playhead(0.999);
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
            egui::Key::O => {
                if command {
                    self.request_action(DeferredAction::OpenSongDialog);
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
                if !self.recent_songs.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("RECENT SONGS")
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                    for entry in self.recent_songs.iter().take(4) {
                        let response = ui.add(
                            egui::Button::new(
                                RichText::new(format!(
                                    "{} • {}",
                                    entry.source_label,
                                    entry.path.display()
                                ))
                                .monospace()
                                .size(11.0)
                                .color(TEXT),
                            )
                            .fill(PANEL_DIM_BG)
                            .stroke(Stroke::new(1.0, BORDER_DIM))
                            .min_size(Vec2::new(ui.available_width(), 20.0)),
                        );
                        if response.clicked() {
                            self.editor_open_path = entry.path.to_string_lossy().to_string();
                            self.active_dialog = Some(ActiveDialog::OpenSongPath);
                        }
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
                    if let Some(track) = self.selected_browser_track() {
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

    fn draw_waveform_panel(&mut self, ui: &mut egui::Ui) {
        let focused = self.runtime.focus == FocusArea::WaveformView;
        Self::retro_panel(
            ui,
            FocusArea::WaveformView.title(),
            focused,
            Some("CLICK OR DRAG TO SEEK • ←/→ NUDGE • HOME/END JUMP"),
            |ui| {
                self.draw_waveform_minimap(ui);
                ui.add_space(6.0);
                let stats = self.current_stats();
                let duration = self.current_render_duration_secs();
                let position_label = self
                    .playback_progress()
                    .map(|p| Self::format_position(p, duration))
                    .unwrap_or_else(|| "-- / --".to_string());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("POS {position_label}"))
                            .monospace()
                            .size(12.0)
                            .color(ACCENT),
                    );
                    ui.separator();
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
            ui.label(
                RichText::new("NO PATTERN BLOCK SELECTED.")
                    .monospace()
                    .color(WARNING),
            );
            return;
        };
        let Some(block) = song.arrangement.blocks.get(block_index) else {
            return;
        };
        if song.tracks.is_empty() {
            ui.label(
                RichText::new("NO TRACKS AVAILABLE.")
                    .monospace()
                    .color(WARNING),
            );
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
                                .color(if track_index == selected_track {
                                    ACCENT
                                } else {
                                    TEXT
                                }),
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
            ui.label(
                RichText::new("TEMPO")
                    .monospace()
                    .size(12.0)
                    .color(TEXT_DIM),
            );
            if ui
                .add(
                    egui::DragValue::new(&mut song.tempo)
                        .range(20..=300)
                        .speed(1),
                )
                .changed()
            {
                marked_dirty = true;
            }
            ui.separator();
            ui.label(
                RichText::new("SWING")
                    .monospace()
                    .size(12.0)
                    .color(TEXT_DIM),
            );
            if ui
                .add(
                    egui::DragValue::new(&mut song.swing)
                        .range(0..=100)
                        .speed(1),
                )
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
            if ui
                .button(RichText::new("-LEN").monospace().size(11.0))
                .clicked()
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
            if ui
                .button(RichText::new("+LEN").monospace().size(11.0))
                .clicked()
            {
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
                ui.label(
                    RichText::new("RENAME")
                        .monospace()
                        .size(12.0)
                        .color(TEXT_DIM),
                );
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.pattern_rename_buffer)
                        .font(TextStyle::Monospace)
                        .desired_width(180.0),
                );
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    self.apply_pattern_rename();
                }
                if ui
                    .button(RichText::new("OK").monospace().size(11.0))
                    .clicked()
                {
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
                    let track_label =
                        format!("{:02} {}", track_index + 1, track.name.to_uppercase());
                    let track_response = ui.add(
                        egui::Button::new(
                            RichText::new(track_label)
                                .monospace()
                                .size(11.0)
                                .color(if track_selected { BASE_BG } else { TEXT }),
                        )
                        .fill(if track_selected { ACCENT } else { PANEL_DIM_BG })
                        .stroke(Stroke::new(
                            1.0,
                            if track_selected { ACCENT } else { BORDER_DIM },
                        ))
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
                            .stroke(Stroke::new(
                                1.0,
                                if is_selected { ACCENT } else { BORDER_DIM },
                            ))
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
            self.set_active_track_index(track_index);
        }
        if let Some(block_index) = clicked_block {
            self.editor_state.selected_arrangement_block = Some(block_index);
            self.sync_arrangement_selection_with_pattern(true);
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
                if self.editor_state.dirty || song_dirty {
                    "YES"
                } else {
                    "NO"
                }
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
            self.invalidate_editable_preview();
        }
    }

    fn ensure_editable_fx_bus_exists(&mut self, bus_index: usize) -> bool {
        let Some(song) = self.editable_song.as_mut() else {
            return false;
        };
        let original_len = song.fx_buses.len();
        while song.fx_buses.len() <= bus_index {
            let next_index = song.fx_buses.len();
            song.fx_buses
                .push(editor::EditableFxBus::default_for_index(next_index));
        }
        song.fx_buses.len() != original_len
    }

    fn waveform_label_from_id(id: i32) -> &'static str {
        match id {
            1 => "pulse",
            2 => "triangle",
            3 => "noise",
            _ => "square",
        }
    }

    fn draw_instrument_inspector(&mut self, ui: &mut egui::Ui) {
        let in_edit_mode =
            self.editor_state.mode != EditorMode::Browser && self.editable_song.is_some();
        Self::retro_panel(
            ui,
            FocusArea::InstrumentInspector.title(),
            self.runtime.focus == FocusArea::InstrumentInspector,
            Some(if in_edit_mode {
                "EDITABLE TRACK VOICE VIEW"
            } else {
                "READ-ONLY VOICE / ADSR VIEW"
            }),
            |ui| {
                if !in_edit_mode {
                    let Some(track) = self.selected_browser_track() else {
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
                    return;
                }

                let mut changed = false;
                let active_track_index = self.active_track_index();

                let (
                    track_name,
                    instrument_label,
                    waveform,
                    duty_cycle,
                    amp,
                    gate,
                    attack,
                    decay,
                    sustain,
                    release,
                    glide,
                    vibrato,
                ) = {
                    let Some(song) = self.editable_song.as_mut() else {
                        return;
                    };
                    if song.tracks.is_empty() || song.instruments.is_empty() {
                        ui.label(
                            RichText::new("NO EDITABLE INSTRUMENT DATA AVAILABLE.")
                                .monospace()
                                .color(WARNING),
                        );
                        return;
                    }

                    let track_index = active_track_index.min(song.tracks.len().saturating_sub(1));
                    let instrument_names: Vec<String> = song
                        .instruments
                        .iter()
                        .map(|instrument| instrument.name.clone())
                        .collect();
                    let track_name = song.tracks[track_index].name.clone();

                    let mut selected_instrument = song.tracks[track_index].instrument_ref.clone();
                    egui::ComboBox::from_label("TRACK INSTRUMENT")
                        .selected_text(selected_instrument.as_str())
                        .show_ui(ui, |ui| {
                            for name in &instrument_names {
                                ui.selectable_value(
                                    &mut selected_instrument,
                                    name.clone(),
                                    name.as_str(),
                                );
                            }
                        });
                    if selected_instrument != song.tracks[track_index].instrument_ref {
                        song.tracks[track_index].instrument_ref = selected_instrument.clone();
                        changed = true;
                    }

                    let instrument_index = song
                        .instruments
                        .iter()
                        .position(|instrument| instrument.name == selected_instrument)
                        .unwrap_or(0);
                    let instrument = &mut song.instruments[instrument_index];
                    let instrument_label = instrument.name.clone();
                    let waveform = Self::waveform_label_from_id(instrument.waveform);
                    let duty_cycle = instrument.duty_cycle;
                    let amp = instrument.amplitude;
                    let gate = instrument.gate_percent;
                    let attack = instrument.attack_ms;
                    let decay = instrument.decay_ms;
                    let sustain = instrument.sustain_level;
                    let release = instrument.release_ms;
                    let glide = instrument.glide_ms;
                    let vibrato = instrument.vibrato_cents;

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("WAVE").monospace().size(12.0).color(TEXT_DIM));
                        egui::ComboBox::from_id_salt("instrument_waveform")
                            .selected_text(waveform.to_uppercase())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut instrument.waveform, 0, "SQUARE");
                                ui.selectable_value(&mut instrument.waveform, 1, "PULSE");
                                ui.selectable_value(&mut instrument.waveform, 2, "TRIANGLE");
                                ui.selectable_value(&mut instrument.waveform, 3, "NOISE");
                            });
                    });
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.amplitude, 1..=127).text("AMP"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.duty_cycle, 1..=100).text("DUTY"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.gate_percent, 1..=100).text("GATE"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.attack_ms, 0..=500).text("ATTACK"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.decay_ms, 0..=500).text("DECAY"))
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut instrument.sustain_level, 0..=100)
                                .text("SUSTAIN"),
                        )
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.release_ms, 0..=600).text("RELEASE"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut instrument.glide_ms, 0..=600).text("GLIDE"))
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut instrument.vibrato_cents, 0..=64)
                                .text("VIBRATO"),
                        )
                        .changed();
                    (
                        track_name,
                        instrument_label,
                        waveform,
                        duty_cycle,
                        amp,
                        gate,
                        attack,
                        decay,
                        sustain,
                        release,
                        glide,
                        vibrato,
                    )
                };

                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "TRACK • {} / {}",
                        track_name.to_uppercase(),
                        instrument_label.to_uppercase()
                    ))
                    .monospace()
                    .strong(),
                );
                ui.horizontal(|ui| {
                    Self::draw_waveform_glyph(ui, waveform, duty_cycle);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        Self::draw_meter_row(
                            ui,
                            "AMP",
                            amp as f32 / 127.0,
                            format!("{amp}"),
                            ACCENT,
                        );
                        Self::draw_meter_row(
                            ui,
                            "DUTY",
                            duty_cycle as f32 / 100.0,
                            format!("{duty_cycle}%"),
                            ACCENT,
                        );
                        Self::draw_meter_row(
                            ui,
                            "GATE",
                            gate as f32 / 100.0,
                            format!("{gate}%"),
                            ACCENT,
                        );
                    });
                });
                Self::draw_meter_row(
                    ui,
                    "ADSR",
                    (attack + decay + release) as f32 / 1500.0,
                    format!("A{attack} D{decay} S{sustain}% R{release}"),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "GLIDE",
                    glide as f32 / 600.0,
                    format!("{glide} ms"),
                    ACCENT,
                );
                Self::draw_meter_row(
                    ui,
                    "VIBRATO",
                    vibrato as f32 / 64.0,
                    format!("{vibrato} c"),
                    ACCENT,
                );

                if changed {
                    self.mark_editor_dirty();
                }
            },
        );
    }

    fn draw_fx_inspector(&mut self, ui: &mut egui::Ui) {
        let in_edit_mode =
            self.editor_state.mode != EditorMode::Browser && self.editable_song.is_some();
        Self::retro_panel(
            ui,
            FocusArea::FxInspector.title(),
            self.runtime.focus == FocusArea::FxInspector,
            Some(if in_edit_mode {
                "EDITABLE BUS / ROUTING VIEW"
            } else {
                "READ-ONLY BUS / FX LANE VIEW"
            }),
            |ui| {
                if !in_edit_mode {
                    let Some(track) = self.selected_browser_track() else {
                        ui.label(
                            RichText::new("NO FX ROUTING AVAILABLE.")
                                .monospace()
                                .color(WARNING),
                        );
                        return;
                    };
                    let Some(bus) = self.selected_browser_fx_bus() else {
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
                    return;
                }

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("EDIT TRACK BUS ROUTING").monospace().strong());
                });

                let mut changed = false;
                let active_track_index = self.active_track_index();

                let (track_name, bus_index_value, bus_values, needs_missing_bus_repair) = {
                    let Some(song) = self.editable_song.as_mut() else {
                        return;
                    };
                    if song.tracks.is_empty() || song.instruments.is_empty() {
                        ui.label(
                            RichText::new("NO EDITABLE FX DATA AVAILABLE.")
                                .monospace()
                                .color(WARNING),
                        );
                        return;
                    }
                    let track_index = active_track_index.min(song.tracks.len().saturating_sub(1));
                    let track_name = song.tracks[track_index].name.clone();
                    let instrument_index = song
                        .instruments
                        .iter()
                        .position(|instrument| {
                            instrument.name == song.tracks[track_index].instrument_ref
                        })
                        .unwrap_or(0);
                    let instrument = &mut song.instruments[instrument_index];

                    let mut bus_index_edit = instrument.fx_bus as i32;
                    if ui
                        .add(egui::Slider::new(&mut bus_index_edit, 0..=31).text("ROUTED BUS"))
                        .changed()
                    {
                        instrument.fx_bus = bus_index_edit.max(0) as usize;
                        changed = true;
                    }
                    let bus_index_value = instrument.fx_bus;

                    if let Some(bus) = song.fx_buses.get_mut(bus_index_value) {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bus.delay_steps, 0..=64).text("DELAY STEPS"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bus.delay_feedback, 0..=100)
                                    .text("DELAY FEEDBACK"),
                            )
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut bus.delay_mix, 0..=100).text("DELAY MIX"))
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut bus.drive_amount, 0..=100).text("DRIVE"))
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bus.lowpass_amount, 0..=100).text("LOWPASS"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bus.sidechain_amount, 0..=100)
                                    .text("SIDECHAIN"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bus.sidechain_release_ms, 20..=800)
                                    .text("SC RELEASE"),
                            )
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut bus.mix_percent, 0..=100).text("BUS MIX"))
                            .changed();
                        (track_name, bus_index_value, bus.clone(), false)
                    } else {
                        (
                            track_name,
                            bus_index_value,
                            editor::EditableFxBus::default_for_index(bus_index_value),
                            true,
                        )
                    }
                };

                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "TRACK {} • BUS {}",
                        track_name.to_uppercase(),
                        bus_index_value
                    ))
                    .monospace()
                    .size(12.0)
                    .color(ACCENT),
                );

                if needs_missing_bus_repair {
                    ui.label(
                        RichText::new(format!("TRACK ROUTES TO MISSING BUS {}.", bus_index_value))
                            .monospace()
                            .color(WARNING),
                    );
                    if ui
                        .button(RichText::new("CREATE MISSING BUS").monospace().size(11.0))
                        .clicked()
                        && self.ensure_editable_fx_bus_exists(bus_index_value)
                    {
                        changed = true;
                    }
                } else {
                    Self::draw_meter_row(
                        ui,
                        "DELAY",
                        (bus_values.delay_mix as f32 / 100.0).clamp(0.0, 1.0),
                        format!(
                            "{} stp / fb {} / mix {}%",
                            bus_values.delay_steps, bus_values.delay_feedback, bus_values.delay_mix
                        ),
                        ACCENT,
                    );
                    Self::draw_meter_row(
                        ui,
                        "DRIVE",
                        (bus_values.drive_amount as f32 / 100.0).clamp(0.0, 1.0),
                        format!("{}%", bus_values.drive_amount),
                        WARNING,
                    );
                    Self::draw_meter_row(
                        ui,
                        "LOW-PASS",
                        (bus_values.lowpass_amount as f32 / 100.0).clamp(0.0, 1.0),
                        format!("{}%", bus_values.lowpass_amount),
                        ACCENT,
                    );
                    Self::draw_meter_row(
                        ui,
                        "SIDECHAIN",
                        (bus_values.sidechain_amount as f32 / 100.0).clamp(0.0, 1.0),
                        format!(
                            "{}% / {} ms",
                            bus_values.sidechain_amount, bus_values.sidechain_release_ms
                        ),
                        ACCENT,
                    );
                    Self::draw_meter_row(
                        ui,
                        "BUS MIX",
                        (bus_values.mix_percent as f32 / 100.0).clamp(0.0, 1.0),
                        format!("{}%", bus_values.mix_percent),
                        ACCENT,
                    );
                }

                if changed {
                    self.mark_editor_dirty();
                }
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
        let selected_song = if self.editor_state.mode == EditorMode::Browser {
            self.selected_demo().key.to_uppercase()
        } else {
            self.editable_song
                .as_ref()
                .map(|song| song.title.to_uppercase())
                .unwrap_or_else(|| "--".to_string())
        };
        let selected_pattern = if self.editor_state.mode == EditorMode::Browser {
            "--".to_string()
        } else {
            self.editable_song
                .as_ref()
                .and_then(|song| {
                    self.editor_state
                        .selected_arrangement_block
                        .and_then(|index| song.arrangement.blocks.get(index))
                        .map(|block| block.pattern_name.to_uppercase())
                })
                .unwrap_or_else(|| "--".to_string())
        };
        let track_label = if self.editor_state.mode == EditorMode::Browser {
            self.selected_browser_track()
                .map(|track| track.name.to_uppercase())
                .unwrap_or_else(|| "--".to_string())
        } else {
            self.selected_editable_track()
                .map(|track| track.name.to_uppercase())
                .unwrap_or_else(|| "--".to_string())
        };
        let dirty_label = if self.editor_state.mode == EditorMode::Browser {
            "NO"
        } else if self.editor_state.dirty
            || self.editable_song.as_ref().is_some_and(|song| song.dirty)
        {
            "YES"
        } else {
            "NO"
        };
        let last_error = self
            .editor_state
            .last_error
            .as_deref()
            .or(self.runtime.last_error.as_deref())
            .unwrap_or("NONE");
        let has_error = self.editor_state.last_error.is_some() || self.runtime.last_error.is_some();
        let current_path = self
            .editable_song
            .as_ref()
            .and_then(|song| song.source_path.as_ref())
            .or(self.editor_state.last_saved_path.as_ref())
            .map(|path| path.display().to_string())
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
                            "SONG {} • PATTERN {} • TRACK {} • FOCUS {}",
                            selected_song,
                            selected_pattern,
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
                        RichText::new(format!("DIRTY {dirty_label}"))
                            .monospace()
                            .size(12.0)
                            .color(if dirty_label == "YES" {
                                WARNING
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
                        RichText::new(format!("LAST ERROR {}", last_error))
                            .monospace()
                            .size(12.0)
                            .color(if has_error { WARNING } else { TEXT_DIM }),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("PATH {}", current_path))
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
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
                        "[CTRL+N/O/S/SHIFT+S/D] SONG FLOW",
                        "[CTRL+R] PREVIEW RENDER",
                        "[W THEN CLICK/DRAG/HOME/END/←→] SEEK PLAYHEAD",
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

    fn draw_active_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.active_dialog.clone() else {
            return;
        };
        match dialog {
            ActiveDialog::OpenSongPath => {
                let mut close_dialog = false;
                egui::Window::new("OPEN SONG")
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(
                            RichText::new("OPEN AN EDITABLE .ABC SONG")
                                .monospace()
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(format!(
                                "USER SONG FOLDER • {}",
                                self.user_song_root.display()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(TEXT_DIM),
                        );
                        ui.add_space(4.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.editor_open_path)
                                .hint_text("ABSOLUTE PATH TO SONG .ABC")
                                .font(TextStyle::Monospace)
                                .desired_width(540.0),
                        );
                        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                            let path = PathBuf::from(self.editor_open_path.trim());
                            self.request_action(DeferredAction::OpenSongPath(path));
                            close_dialog = true;
                        }
                        if !self.recent_songs.is_empty() {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.label(RichText::new("RECENT").monospace().size(11.0).color(TEXT_DIM));
                            for entry in self.recent_songs.iter().take(5) {
                                if ui
                                    .button(
                                        RichText::new(format!(
                                            "{} • {}",
                                            entry.source_label,
                                            entry.path.display()
                                        ))
                                        .monospace()
                                        .size(11.0),
                                    )
                                    .clicked()
                                {
                                    self.editor_open_path = entry.path.to_string_lossy().to_string();
                                }
                            }
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("OPEN").monospace().size(11.0)).clicked() {
                                let path = PathBuf::from(self.editor_open_path.trim());
                                self.request_action(DeferredAction::OpenSongPath(path));
                                close_dialog = true;
                            }
                            if ui.button(RichText::new("CANCEL (ESC)").monospace().size(11.0)).clicked()
                            {
                                close_dialog = true;
                            }
                        });
                    });
                if close_dialog && matches!(self.active_dialog, Some(ActiveDialog::OpenSongPath)) {
                    self.active_dialog = None;
                }
            }
            ActiveDialog::SaveAsPath => {
                let mut close_dialog = false;
                egui::Window::new("SAVE SONG AS")
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(
                            RichText::new("SAVE EDITABLE SONG TO .ABC")
                                .monospace()
                                .size(12.0),
                        );
                        ui.label(
                            RichText::new(format!(
                                "DEFAULT USER FOLDER • {}",
                                self.user_song_root.display()
                            ))
                            .monospace()
                            .size(11.0)
                            .color(TEXT_DIM),
                        );
                        ui.add_space(4.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.editor_open_path)
                                .hint_text("ABSOLUTE PATH TO SAVE .ABC")
                                .font(TextStyle::Monospace)
                                .desired_width(540.0),
                        );
                        if response.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                            && self.save_editable_song(true)
                        {
                            close_dialog = true;
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("SAVE").monospace().size(11.0)).clicked()
                                && self.save_editable_song(true)
                            {
                                close_dialog = true;
                            }
                            if ui.button(RichText::new("CANCEL (ESC)").monospace().size(11.0)).clicked()
                            {
                                close_dialog = true;
                            }
                        });
                    });
                if close_dialog && matches!(self.active_dialog, Some(ActiveDialog::SaveAsPath)) {
                    self.active_dialog = None;
                }
            }
            ActiveDialog::UnsavedChanges { .. } => {
                let mut cancel = false;
                egui::Window::new("UNSAVED CHANGES")
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(
                            RichText::new("YOU HAVE UNSAVED SONG CHANGES.")
                                .monospace()
                                .size(12.0)
                                .color(WARNING),
                        );
                        ui.label(
                            RichText::new("SAVE / DISCARD / CANCEL")
                                .monospace()
                                .size(11.0)
                                .color(TEXT_DIM),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("SAVE (S)").monospace().size(11.0)).clicked() {
                                self.resolve_unsaved_action(true, false);
                            }
                            if ui
                                .button(RichText::new("DISCARD (D)").monospace().size(11.0))
                                .clicked()
                            {
                                self.resolve_unsaved_action(false, true);
                            }
                            if ui
                                .button(RichText::new("CANCEL (ESC)").monospace().size(11.0))
                                .clicked()
                            {
                                cancel = true;
                            }
                        });
                    });
                if cancel {
                    self.active_dialog = None;
                }
            }
        }
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        let song_label = if self.editor_state.mode == EditorMode::Browser {
            self.selected_demo().key.to_uppercase()
        } else {
            self.editable_song
                .as_ref()
                .map(|song| song.title.to_uppercase())
                .unwrap_or_else(|| "UNTITLED".to_string())
        };
        let dirty_prefix = if self.editor_state.mode != EditorMode::Browser && self.effective_song_dirty()
        {
            "* "
        } else {
            ""
        };
        let title = format!(
            "{dirty_prefix}MEMDECK SOUND MACHINE • {} • {}",
            self.editor_state.mode.label(),
            song_label
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
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

    fn draw_waveform_minimap(&mut self, ui: &mut egui::Ui) {
        let samples = self.current_render().map(|render| render.samples.clone());
        let stats = self.current_stats();
        let progress = self.playback_progress();
        let duration = self.current_render_duration_secs();
        let has_render = samples.is_some();

        let desired_size = egui::vec2(ui.available_width(), 156.0);
        let (response, painter) = ui.allocate_painter(
            desired_size,
            if has_render {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        let rect = response.rect.shrink(4.0);

        painter.rect_filled(rect, 0.0, PANEL_DIM_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, BORDER_DIM));

        // Interaction — translate pointer x into [0,1] across the rect.
        let pointer_progress = |pos: egui::Pos2| -> f32 {
            ((pos.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 0.999)
        };
        let mut seek_to: Option<f32> = None;
        if has_render {
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let p = pointer_progress(pos);
                    // Live visual preview only — audio respawns on release.
                    self.runtime.waveform_drag_preview = Some(p);
                }
            }
            if response.drag_stopped() {
                if let Some(p) = self.runtime.waveform_drag_preview.take() {
                    seek_to = Some(p);
                }
            }
            // A pure click (no drag) registers via clicked(); commit immediately.
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.runtime.waveform_drag_preview = None;
                    seek_to = Some(pointer_progress(pos));
                }
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }

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
        let samples_slice: &[u8] = samples.as_ref();

        let columns = rect.width().max(64.0) as usize;
        let stride = (samples_slice.len() / columns).max(1);
        let usable_width = (rect.width() - 8.0).max(1.0);
        let mut upper = Vec::with_capacity(columns);
        let mut lower = Vec::with_capacity(columns);
        let mut clip_markers = Vec::with_capacity(columns / 4);

        for (index, chunk) in samples_slice.chunks(stride).take(columns).enumerate() {
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

        // Hover preview (ghost playhead) — only when not actively dragging.
        if response.hovered() && !response.dragged() {
            if let Some(hover) = response.hover_pos() {
                let hover_progress = pointer_progress(hover);
                let x = egui::lerp(rect.left()..=rect.right(), hover_progress);
                painter.line_segment(
                    [egui::pos2(x, rect.top() + 2.0), egui::pos2(x, rect.bottom() - 2.0)],
                    Stroke::new(1.0, TEXT_DIM),
                );
                let label = Self::format_position(hover_progress, duration);
                painter.text(
                    egui::pos2(x + 6.0, rect.top() + 4.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::monospace(10.0),
                    TEXT_DIM,
                );
            }
        }

        if let Some(progress) = progress {
            let x = egui::lerp(rect.left()..=rect.right(), progress.clamp(0.0, 1.0));
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(2.0, ACCENT),
            );
            // Triangular handle on top so the playhead reads as graspable.
            let handle = [
                egui::pos2(x - 5.0, rect.top()),
                egui::pos2(x + 5.0, rect.top()),
                egui::pos2(x, rect.top() + 8.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                handle.to_vec(),
                ACCENT,
                Stroke::NONE,
            ));
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

        if let Some(target) = seek_to {
            self.seek_playhead(target);
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
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && self.active_dialog.is_none() {
            if self.editor_state.mode != EditorMode::Browser && self.effective_song_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.open_unsaved_changes_dialog(DeferredAction::QuitApplication);
            } else {
                self.request_quit = true;
            }
        }

        self.poll_playback();
        self.apply_boot_options();
        self.update_window_title(ctx);
        self.handle_keyboard(ctx);
        self.sync_track_selection_state();

        if self.playback.is_playing() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }

        egui::TopBottomPanel::top("runtime_header").show(ctx, |ui| {
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
                    let header_song = if self.editor_state.mode == EditorMode::Browser {
                        self.selected_demo().key.to_uppercase()
                    } else {
                        self.editable_song
                            .as_ref()
                            .map(|song| song.title.to_uppercase())
                            .unwrap_or_else(|| self.selected_demo().key.to_uppercase())
                    };
                    ui.label(
                        RichText::new(format!("SONG • {header_song}"))
                            .monospace()
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                    let header_track = if self.editor_state.mode == EditorMode::Browser {
                        self.selected_browser_track()
                            .map(|track| track.name.to_uppercase())
                    } else {
                        self.selected_editable_track()
                            .map(|track| track.name.to_uppercase())
                    };
                    if let Some(track_name) = header_track {
                        ui.separator();
                        ui.label(
                            RichText::new(format!("TRACK • {track_name}"))
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
                        .color(
                            if self.editor_state.mode == EditorMode::Browser {
                                BORDER
                            } else {
                                ACCENT
                            },
                        ),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(RichText::new("NEW SONG").monospace().size(12.0))
                        .clicked()
                    {
                        self.request_action(DeferredAction::NewSong);
                    }
                    if ui
                        .button(
                            RichText::new("DUPLICATE DEMO AS EDITABLE")
                                .monospace()
                                .size(12.0),
                        )
                        .clicked()
                    {
                        self.request_action(DeferredAction::DuplicateFromBrowserDemo);
                    }
                    if ui
                        .button(RichText::new("OPEN SONG").monospace().size(12.0))
                        .clicked()
                    {
                        self.request_action(DeferredAction::OpenSongDialog);
                    }
                    if self.editor_state.mode != EditorMode::Browser
                        && ui
                            .button(RichText::new("SAVE").monospace().size(12.0))
                            .clicked()
                    {
                        let _ = self.save_editable_song(false);
                    }
                    if self.editor_state.mode != EditorMode::Browser
                        && ui
                            .button(RichText::new("SAVE AS").monospace().size(12.0))
                            .clicked()
                    {
                        self.active_dialog = Some(ActiveDialog::SaveAsPath);
                    }
                    if self.editor_state.mode != EditorMode::Browser
                        && ui
                            .button(RichText::new("CLOSE SONG").monospace().size(12.0))
                            .clicked()
                    {
                        self.request_action(DeferredAction::CloseSong);
                    }
                    if self.editor_state.mode != EditorMode::Browser
                        && ui
                            .button(RichText::new("BROWSER MODE").monospace().size(12.0))
                            .clicked()
                    {
                        self.request_action(DeferredAction::SwitchMode(EditorMode::Browser));
                    }
                    if self.editor_state.mode == EditorMode::Preview
                        && ui
                            .button(RichText::new("EDIT MODE").monospace().size(12.0))
                            .clicked()
                    {
                        self.set_mode(EditorMode::Edit);
                    }
                    if self.editor_state.mode == EditorMode::Edit
                        && ui
                            .button(RichText::new("PREVIEW").monospace().size(12.0))
                            .clicked()
                    {
                        self.render_editable_preview();
                    }
                    if !self.editor_open_path.trim().is_empty() {
                        ui.label(
                            RichText::new(format!("PATH • {}", self.editor_open_path))
                                .monospace()
                                .size(11.0)
                                .color(TEXT_DIM),
                        );
                    }
                });
            });

        egui::TopBottomPanel::bottom("runtime_footer")
            .resizable(false)
            .show(ctx, |ui| self.draw_status_line(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let is_portrait = available.y > available.x * 1.1;
            let use_compact_layout = is_portrait || available.x < 1180.0;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        if use_compact_layout {
                            self.draw_demo_browser(ui);
                            ui.add_space(8.0);
                            self.draw_stats_panel(ui);
                        } else {
                            ui.columns(2, |columns| {
                                self.draw_demo_browser(&mut columns[0]);
                                self.draw_stats_panel(&mut columns[1]);
                            });
                        }
                        ui.add_space(8.0);
                        self.draw_waveform_panel(ui);
                        ui.add_space(8.0);
                        self.draw_pattern_panel(ui);
                        ui.add_space(8.0);
                        self.draw_pattern_editor_panel(ui);
                        ui.add_space(8.0);
                        if use_compact_layout {
                            self.draw_instrument_inspector(ui);
                            ui.add_space(8.0);
                            self.draw_fx_inspector(ui);
                        } else {
                            ui.columns(2, |columns| {
                                self.draw_instrument_inspector(&mut columns[0]);
                                self.draw_fx_inspector(&mut columns[1]);
                            });
                        }
                    });
                });
        });
        self.draw_active_dialog(ctx);
        if self.request_quit {
            self.request_quit = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
            editable_source: env::var("MEMDECK_GUI_BOOT_EDITABLE")
                .ok()
                .and_then(|value| {
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
            dialog: env::var("MEMDECK_GUI_BOOT_DIALOG")
                .ok()
                .and_then(|value| {
                    let normalized = value.to_lowercase();
                    if normalized == "open" || normalized == "save-as" {
                        Some(normalized)
                    } else {
                        None
                    }
                }),
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
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
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
        app.sync_track_selection_state();

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

    fn install_dummy_render(app: &mut MemDeckGuiApp) {
        let key = app.demos[app.runtime.selected_demo].key.clone();
        app.runtime.rendered_audio = Some(RenderState {
            demo_key: key,
            samples: Arc::<[u8]>::from(vec![128_u8; 22050]), // 1.0s of silent PCM
            stats: Some(AudioRenderStats {
                sample_count: 22050,
                duration_ms: 1000.0,
                min_sample: 0,
                max_sample: 0,
                peak: 0,
                clipping_count: 0,
                checksum: 0,
                render_time_ms: 0.0,
            }),
        });
    }

    #[test]
    fn seek_playhead_updates_position_when_stopped() {
        let mut app = MemDeckGuiApp::default();
        install_dummy_render(&mut app);

        app.seek_playhead(0.5);
        assert!(
            (app.runtime.playhead_position - 0.5).abs() < f32::EPSILON,
            "playhead should land at 0.5, got {}",
            app.runtime.playhead_position
        );
        assert_eq!(
            app.playback_progress(),
            Some(0.5),
            "stopped playback should expose the user-managed playhead"
        );
    }

    #[test]
    fn seek_playhead_clamps_to_safe_range() {
        let mut app = MemDeckGuiApp::default();
        install_dummy_render(&mut app);

        app.seek_playhead(2.0);
        assert!(
            app.runtime.playhead_position < 1.0,
            "playhead should be clamped strictly below 1.0 to avoid empty tail playback"
        );

        app.seek_playhead(-1.0);
        assert!(
            (app.runtime.playhead_position - 0.0).abs() < f32::EPSILON,
            "negative seek should clamp to 0"
        );
    }

    #[test]
    fn seek_playhead_ignored_without_render() {
        let mut app = MemDeckGuiApp::default();
        app.runtime.rendered_audio = None;
        app.runtime.playhead_position = 0.3;

        app.seek_playhead(0.8);
        assert!(
            (app.runtime.playhead_position - 0.3).abs() < f32::EPSILON,
            "seek should be a no-op when no PCM is loaded"
        );
        assert_eq!(app.playback_progress(), None);
    }

    #[test]
    fn nudge_playhead_uses_actual_duration() {
        let mut app = MemDeckGuiApp::default();
        install_dummy_render(&mut app);
        app.seek_playhead(0.5); // 0.5s into a 1.0s clip

        app.nudge_playhead_seconds(-0.25);
        assert!(
            (app.runtime.playhead_position - 0.25).abs() < 1e-3,
            "1s clip with -0.25s nudge should land near 0.25, got {}",
            app.runtime.playhead_position
        );

        // Large positive nudge clamps inside the clip rather than overshooting.
        app.nudge_playhead_seconds(100.0);
        assert!(
            app.runtime.playhead_position < 1.0,
            "nudge must respect the playhead clamp"
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
        assert!(
            demo_count >= 2,
            "expected at least two demos for navigation"
        );

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

    #[test]
    fn mode_transitions_preserve_edit_selection() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        app.arrangement_add_block();
        app.editor_state.selected_arrangement_block = Some(1);
        app.sync_arrangement_selection_with_pattern(false);
        app.set_active_track_index(0);
        app.editor_state.selected_step = Some(16);
        app.toggle_selected_step();

        app.render_editable_preview();
        assert_eq!(app.editor_state.mode, EditorMode::Preview);
        app.set_mode(EditorMode::Edit);
        assert_eq!(app.editor_state.mode, EditorMode::Edit);
        assert_eq!(app.editor_state.selected_arrangement_block, Some(1));
        assert_eq!(app.editor_state.selected_step, Some(16));

        app.set_mode(EditorMode::Browser);
        assert_eq!(app.editor_state.mode, EditorMode::Browser);
        app.set_mode(EditorMode::Edit);
        assert_eq!(app.editor_state.selected_arrangement_block, Some(1));
        assert_eq!(app.editor_state.selected_step, Some(16));
    }

    #[test]
    fn arrangement_selection_updates_pattern_target() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();

        app.arrangement_add_block();
        assert_eq!(app.editor_state.selected_arrangement_block, Some(1));
        assert_eq!(app.editor_state.selected_pattern, Some(1));

        app.arrangement_move_cursor(-1);
        assert_eq!(app.editor_state.selected_arrangement_block, Some(0));
        assert_eq!(app.editor_state.selected_pattern, Some(0));
    }

    #[test]
    fn editing_while_previewing_invalidates_render_and_returns_edit_mode() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        app.toggle_selected_step();
        app.render_editable_preview();
        assert_eq!(app.editor_state.mode, EditorMode::Preview);
        assert!(app.current_render().is_some());

        app.toggle_selected_step();
        assert_eq!(app.editor_state.mode, EditorMode::Edit);
        assert!(app.current_render().is_none());
    }

    #[test]
    fn invalid_serialization_sets_visible_error() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        if let Some(song) = app.editable_song.as_mut() {
            song.patterns.clear();
            song.arrangement.blocks.clear();
        }

        app.render_editable_preview();
        assert_eq!(app.editor_state.mode, EditorMode::Edit);
        assert!(
            app.editor_state.last_error.is_some(),
            "render failure should expose serialization error"
        );
    }

    struct TempSongFile(PathBuf);

    impl TempSongFile {
        fn new(label: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "memdeck-gui-{label}-{}.abc",
                std::process::id()
            )))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempSongFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn new_song_initializes_editable_lifecycle_state() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        assert_eq!(app.editor_state.mode, EditorMode::Edit);
        assert!(app.editable_song.is_some());
        assert!(!app.effective_song_dirty());
        assert!(!app.editor_open_path.trim().is_empty());
    }

    #[test]
    fn save_after_edit_clears_dirty_state() {
        let mut app = MemDeckGuiApp::default();
        app.user_song_root = std::env::temp_dir().join("memdeck-gui-save-after-edit");
        app.create_new_song();
        app.toggle_selected_step();
        assert!(app.effective_song_dirty());

        let saved = app.save_editable_song(false);
        assert!(saved, "save should succeed");
        assert!(!app.effective_song_dirty(), "save should clear dirty state");
    }

    #[test]
    fn open_song_roundtrip_from_saved_file() {
        let mut app = MemDeckGuiApp::default();
        app.user_song_root = std::env::temp_dir().join("memdeck-gui-roundtrip");
        app.create_new_song();
        if let Some(song) = app.editable_song.as_mut() {
            song.title = "Roundtrip".to_string();
        }
        let path = TempSongFile::new("open-roundtrip");
        app.editor_open_path = path.path().to_string_lossy().to_string();
        assert!(app.save_editable_song(true));

        app.close_current_song();
        app.open_editable_song_from_path(path.path().to_path_buf());
        assert_eq!(app.editor_state.mode, EditorMode::Edit);
        assert_eq!(
            app.editable_song.as_ref().map(|song| song.title.as_str()),
            Some("Roundtrip")
        );
    }

    #[test]
    fn invalid_save_path_surfaces_error() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        app.editor_open_path = "/dev/null/memdeck-invalid.abc".to_string();
        let saved = app.save_editable_song(true);
        assert!(!saved, "save should fail for invalid path");
        assert!(app.editor_state.last_error.is_some());
    }

    #[test]
    fn recent_songs_tracks_open_and_duplicate_entries() {
        let mut app = MemDeckGuiApp::default();
        app.duplicate_demo_as_editable();
        assert!(
            !app.recent_songs.is_empty(),
            "duplicate demo should appear in recents"
        );

        let path = TempSongFile::new("recent-open");
        let content = [
            "X:1",
            "T:Recent",
            "M:4/4",
            "L:1/16",
            "Q:1/4=120",
            "K:C",
            "%%instrument lead wave=square amp=64 duty=25 attack=0 decay=0 sustain=100 release=0 gate=90 fx=0",
            "%%effect 0 delay_steps=0 delay_feedback=0 delay_mix=0 drive=0 lowpass=0 sidechain=0 sidechain_release=180 mix=100",
            "%%pattern A length=16",
            "%%arrangement A",
            "V:lead instrument=lead",
            "V:lead",
            "| czzzczzzczzzczzz |",
        ]
        .join("\n");
        fs::write(path.path(), content).expect("fixture should write");
        app.open_editable_song_from_path(path.path().to_path_buf());
        assert!(
            app.recent_songs
                .iter()
                .any(|entry| {
                    entry.path.as_path() == path.path() && entry.source_label.contains("OPENED")
                }),
            "opened song should be tracked in recents"
        );
    }

    #[test]
    fn unsaved_changes_dialog_blocks_navigation_until_resolved() {
        let mut app = MemDeckGuiApp::default();
        app.create_new_song();
        app.toggle_selected_step();
        app.request_action(DeferredAction::CloseSong);
        assert!(
            matches!(app.active_dialog, Some(ActiveDialog::UnsavedChanges { .. })),
            "dirty close request should trigger unsaved dialog"
        );
    }
}
