use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;
use std::slice;
use std::sync::{LazyLock, Mutex};

use crate::audio_engine::{DemoOverview, FxBusOverview, PatternBlock, StepState, TrackOverview};

pub const SAMPLE_RATE_ABC: c_int = 22_050;
const ABC_DEFAULT_VIBRATO_RATE: c_int = 5_200;
const ABC_DEFAULT_SIDECHAIN_RELEASE_MS: c_int = 180;
const ABC_MAX_VOICES: usize = 8;
const ABC_MAX_NOTES: usize = 1024;
const ABC_MAX_INSTRUMENTS: usize = 8;
const ABC_MAX_PATTERNS: usize = 16;
const ABC_MAX_ARRANGEMENT: usize = 32;
const ABC_MAX_FX_BUSES: usize = 4;
const SEQ_MAX_TRACKS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioRenderStats {
    pub sample_count: u64,
    pub duration_ms: f64,
    pub min_sample: c_int,
    pub max_sample: c_int,
    pub peak: c_int,
    pub clipping_count: u64,
    pub checksum: u64,
    pub render_time_ms: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AbcInstrument {
    name: [c_char; 32],
    preset: [c_char; 32],
    amplitude: c_int,
    waveform: c_int,
    duty_cycle: c_int,
    attack_ms: c_int,
    decay_ms: c_int,
    sustain_level: c_int,
    release_ms: c_int,
    gate_percent: c_int,
    vibrato_cents: c_int,
    glide_ms: c_int,
    fx_bus: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AbcPatternDef {
    name: [c_char; 32],
    length: c_int,
    defined: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AbcFxBus {
    enabled: c_int,
    delay_steps: c_int,
    delay_feedback: c_int,
    delay_mix: c_int,
    drive_amount: c_int,
    lowpass_amount: c_int,
    sidechain_amount: c_int,
    sidechain_release_ms: c_int,
    mix_percent: c_int,
    ladder_amount: c_int,
    ladder_cutoff: c_int,
    ladder_resonance: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AbcVoice {
    name: [c_char; 32],
    instrument_ref: [c_char; 32],
    amplitude: c_int,
    staccato: c_int,
    waveform: c_int,
    duty_cycle: c_int,
    attack_ms: c_int,
    decay_ms: c_int,
    sustain_level: c_int,
    release_ms: c_int,
    gate_percent: c_int,
    vibrato_cents: c_int,
    vibrato_rate: c_int,
    glide_ms: c_int,
    fx_bus: c_int,
    freqs: [c_double; ABC_MAX_NOTES],
    note_count: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AbcMusic {
    title: [c_char; 128],
    bpm: c_int,
    step_ms: c_int,
    swing_pct: c_int,
    instruments: [AbcInstrument; ABC_MAX_INSTRUMENTS],
    instrument_count: c_int,
    patterns: [AbcPatternDef; ABC_MAX_PATTERNS],
    pattern_count: c_int,
    arrangement: [[c_char; 32]; ABC_MAX_ARRANGEMENT],
    arrangement_length: c_int,
    fx_buses: [AbcFxBus; ABC_MAX_FX_BUSES],
    fx_bus_count: c_int,
    fx_delay_steps: c_int,
    fx_delay_feedback: c_int,
    fx_delay_mix: c_int,
    fx_drive_amount: c_int,
    fx_lowpass_amount: c_int,
    fx_sidechain_amount: c_int,
    fx_sidechain_release_ms: c_int,
    voices: [AbcVoice; ABC_MAX_VOICES],
    voice_count: c_int,
}

impl Default for AbcVoice {
    fn default() -> Self {
        Self {
            name: [0; 32],
            instrument_ref: [0; 32],
            amplitude: 0,
            staccato: 0,
            waveform: 0,
            duty_cycle: 0,
            attack_ms: 0,
            decay_ms: 0,
            sustain_level: 0,
            release_ms: 0,
            gate_percent: 0,
            vibrato_cents: 0,
            vibrato_rate: 0,
            glide_ms: 0,
            fx_bus: 0,
            freqs: [0.0; ABC_MAX_NOTES],
            note_count: 0,
        }
    }
}

impl Default for AbcMusic {
    fn default() -> Self {
        Self {
            title: [0; 128],
            bpm: 0,
            step_ms: 0,
            swing_pct: 0,
            instruments: [AbcInstrument::default(); ABC_MAX_INSTRUMENTS],
            instrument_count: 0,
            patterns: [AbcPatternDef::default(); ABC_MAX_PATTERNS],
            pattern_count: 0,
            arrangement: [[0; 32]; ABC_MAX_ARRANGEMENT],
            arrangement_length: 0,
            fx_buses: [AbcFxBus::default(); ABC_MAX_FX_BUSES],
            fx_bus_count: 0,
            fx_delay_steps: 0,
            fx_delay_feedback: 0,
            fx_delay_mix: 0,
            fx_drive_amount: 0,
            fx_lowpass_amount: 0,
            fx_sidechain_amount: 0,
            fx_sidechain_release_ms: 0,
            voices: [AbcVoice::default(); ABC_MAX_VOICES],
            voice_count: 0,
        }
    }
}

unsafe extern "C" {
    fn audio_engine_render_abc_file(
        path: *const c_char,
        sample_rate: c_int,
        out_len: *mut c_int,
        out_stats: *mut AudioRenderStats,
    ) -> *mut u8;

    fn audio_engine_free_buffer(buffer: *mut u8);
    fn abc_load(path: *const c_char, music: *mut AbcMusic) -> c_int;

    fn audio_jam_session_open(
        abc_path: *const c_char,
        seed: u64,
        section_seconds: c_double,
    ) -> *mut AudioJamSessionRaw;
    fn audio_jam_session_render_next(
        session: *mut AudioJamSessionRaw,
        sample_rate: c_int,
        out_pcm_len: *mut c_int,
    ) -> *mut u8;
    fn audio_jam_session_iteration(session: *const AudioJamSessionRaw) -> c_int;
    fn audio_jam_session_arrangement_offset(session: *const AudioJamSessionRaw) -> c_int;
    fn audio_jam_session_slots_per_section(session: *const AudioJamSessionRaw) -> c_int;
    fn audio_jam_session_close(session: *mut AudioJamSessionRaw);
}

#[repr(C)]
struct AudioJamSessionRaw {
    _private: [u8; 0],
}

static LAST_RENDER_STATS: LazyLock<Mutex<Option<AudioRenderStats>>> =
    LazyLock::new(|| Mutex::new(None));

struct RenderedBuffer {
    ptr: *mut u8,
    len: usize,
}

impl RenderedBuffer {
    fn from_raw(ptr: *mut u8, len: c_int) -> Result<Self, String> {
        if ptr.is_null() {
            return Err("render failure: engine returned a null PCM buffer".to_string());
        }
        if len <= 0 {
            free_buffer(ptr);
            return Err("render failure: engine returned an empty PCM buffer".to_string());
        }

        Ok(Self {
            ptr,
            len: len as usize,
        })
    }

    fn into_vec(mut self) -> Vec<u8> {
        let samples = unsafe {
            let slice = slice::from_raw_parts(self.ptr as *const u8, self.len);
            slice.to_vec()
        };
        self.release();
        samples
    }

    fn release(&mut self) {
        if !self.ptr.is_null() {
            free_buffer(self.ptr);
            self.ptr = ptr::null_mut();
        }
    }
}

impl Drop for RenderedBuffer {
    fn drop(&mut self) {
        self.release();
    }
}

fn normalized_demo_path(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("missing demo file: {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("demo path is not a file: {}", path.display()));
    }

    let path_str = path
        .to_str()
        .ok_or_else(|| format!("demo path is not valid UTF-8: {}", path.display()))?;
    Ok(path_str.to_string())
}

pub fn render_abc_file(path: &Path) -> Result<Vec<u8>, String> {
    let path_str = normalized_demo_path(path)?;
    let c_path = CString::new(path_str.clone())
        .map_err(|_| format!("demo path contains NUL byte: {}", path.display()))?;

    let render_result = catch_unwind(|| {
        let mut pcm_len = 0;
        let mut stats = AudioRenderStats::default();
        let buffer = unsafe {
            audio_engine_render_abc_file(c_path.as_ptr(), SAMPLE_RATE_ABC, &mut pcm_len, &mut stats)
        };
        (buffer, pcm_len, stats)
    });

    let (buffer, pcm_len, stats) = render_result.map_err(|_| {
        format!(
            "render failure: engine call panicked for {}",
            path.display()
        )
    })?;
    let samples = RenderedBuffer::from_raw(buffer, pcm_len)?.into_vec();

    if let Ok(mut last_stats) = LAST_RENDER_STATS.lock() {
        *last_stats = Some(stats);
    }

    Ok(samples)
}

/// Owned handle to a C-side jam session. Each `render_next()` call produces
/// a fresh ~N-second PCM section (varied via [`audio_jam_vary_song`] under
/// the hood) and advances the session's scroll head. Drop closes the C
/// session; the underlying SeqSong copy is freed there.
pub struct JamSession {
    raw: *mut AudioJamSessionRaw,
}

unsafe impl Send for JamSession {}

pub struct JamSection {
    pub samples: Vec<u8>,
    pub iteration: u32,
    #[allow(dead_code)] // exposed for future status-bar UI / debugging
    pub arrangement_offset: i32,
    #[allow(dead_code)]
    pub slots_per_section: i32,
}

impl JamSession {
    pub fn open(path: &Path, seed: u64, section_seconds: f64) -> Result<Self, String> {
        let path_str = normalized_demo_path(path)?;
        let c_path = CString::new(path_str.clone())
            .map_err(|_| format!("demo path contains NUL byte: {}", path.display()))?;

        let raw = catch_unwind(AssertUnwindSafe(|| unsafe {
            audio_jam_session_open(c_path.as_ptr(), seed, section_seconds)
        }))
        .map_err(|_| format!("jam session open panicked for {}", path.display()))?;

        if raw.is_null() {
            return Err(format!(
                "could not open jam session for {} (parse/build failed or empty arrangement)",
                path.display()
            ));
        }
        Ok(Self { raw })
    }

    pub fn render_next(&mut self) -> Result<JamSection, String> {
        if self.raw.is_null() {
            return Err("jam session is closed".to_string());
        }
        let mut pcm_len: c_int = 0;

        let ptr = catch_unwind(AssertUnwindSafe(|| unsafe {
            audio_jam_session_render_next(self.raw, SAMPLE_RATE_ABC, &mut pcm_len)
        }))
        .map_err(|_| "jam render panicked".to_string())?;

        if ptr.is_null() || pcm_len <= 0 {
            if !ptr.is_null() {
                free_buffer(ptr);
            }
            return Err("jam render returned empty PCM".to_string());
        }
        let samples = unsafe { slice::from_raw_parts(ptr, pcm_len as usize).to_vec() };
        free_buffer(ptr);

        let (iteration, offset, slots) = unsafe {
            (
                audio_jam_session_iteration(self.raw),
                audio_jam_session_arrangement_offset(self.raw),
                audio_jam_session_slots_per_section(self.raw),
            )
        };
        Ok(JamSection {
            samples,
            iteration: iteration.max(0) as u32,
            arrangement_offset: offset,
            slots_per_section: slots,
        })
    }
}

impl Drop for JamSession {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                audio_jam_session_close(self.raw);
            }
            self.raw = std::ptr::null_mut();
        }
    }
}

pub fn load_demo_overview(path: &Path) -> Result<DemoOverview, String> {
    let path_str = normalized_demo_path(path)?;
    let c_path = CString::new(path_str)
        .map_err(|_| format!("demo path contains NUL byte: {}", path.display()))?;
    let mut music = AbcMusic::default();

    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        abc_load(c_path.as_ptr(), &mut music)
    }))
    .map_err(|_| format!("invalid ABC: parser panicked for {}", path.display()))?;
    if result != 0 {
        return Err(format!(
            "invalid ABC or unreadable demo: {}",
            path.display()
        ));
    }

    build_demo_overview(&music)
}

pub fn free_buffer(buffer: *mut u8) {
    if !buffer.is_null() {
        unsafe {
            audio_engine_free_buffer(buffer);
        }
    }
}

pub fn get_render_stats() -> Option<AudioRenderStats> {
    LAST_RENDER_STATS.lock().ok().and_then(|stats| *stats)
}

fn build_demo_overview(music: &AbcMusic) -> Result<DemoOverview, String> {
    let preset_map = build_instrument_preset_map(music);
    let total_tracks = music.voice_count.max(0) as usize;
    let visible_tracks = usize::min(total_tracks, SEQ_MAX_TRACKS);
    let hidden_track_count = total_tracks - visible_tracks;
    let max_steps = music.voices[..visible_tracks]
        .iter()
        .map(|voice| voice.note_count.max(0) as usize)
        .max()
        .unwrap_or(0);
    let steps_per_beat = inferred_steps_per_beat(music);

    let arrangement = build_arrangement(music, max_steps)?;
    let total_steps = arrangement
        .last()
        .map(|block| block.start_step + block.length)
        .unwrap_or(max_steps);

    let tracks = music.voices[..visible_tracks]
        .iter()
        .enumerate()
        .map(|(index, voice)| {
            build_track_overview(index, voice, &preset_map, total_steps, steps_per_beat)
        })
        .collect();

    Ok(DemoOverview {
        title: c_buf_to_string(&music.title),
        bpm: music.bpm,
        swing_pct: if music.swing_pct <= 0 {
            50
        } else {
            music.swing_pct
        },
        steps_per_beat,
        total_steps: total_steps.max(max_steps),
        arrangement,
        tracks,
        fx_buses: build_fx_buses(music),
        hidden_track_count,
    })
}

fn build_track_overview(
    index: usize,
    voice: &AbcVoice,
    preset_map: &HashMap<String, String>,
    total_steps: usize,
    steps_per_beat: usize,
) -> TrackOverview {
    let name = c_buf_to_string(&voice.name);
    let instrument_ref = c_buf_to_string(&voice.instrument_ref);
    let display_name = if name.is_empty() {
        format!("track_{:02}", index + 1)
    } else {
        name
    };
    let preset = preset_map.get(&instrument_ref).cloned().unwrap_or_default();
    let instrument = if instrument_ref.is_empty() {
        "direct".to_string()
    } else {
        instrument_ref
    };

    let mut activity = vec![StepState::default(); total_steps];
    for (step_index, &frequency) in voice
        .freqs
        .iter()
        .take(voice.note_count.max(0) as usize)
        .enumerate()
    {
        if step_index >= activity.len() || frequency <= 0.0 {
            continue;
        }

        let accent = step_index % steps_per_beat == 0;
        activity[step_index] = StepState {
            active: true,
            accent,
            fx_trigger: accent,
        };
    }

    TrackOverview {
        name: display_name,
        instrument,
        preset,
        waveform: waveform_name(voice.waveform).to_string(),
        amplitude: voice.amplitude,
        duty_cycle: voice.duty_cycle,
        attack_ms: voice.attack_ms,
        decay_ms: voice.decay_ms,
        sustain_level: voice.sustain_level,
        release_ms: voice.release_ms,
        gate_percent: voice.gate_percent,
        vibrato_cents: voice.vibrato_cents,
        vibrato_rate: if voice.vibrato_rate > 0 {
            voice.vibrato_rate
        } else {
            ABC_DEFAULT_VIBRATO_RATE
        },
        glide_ms: voice.glide_ms,
        // The ABC GUI metadata path does not expose detune; keep the inspector explicit
        // about the current read-only value instead of inventing a synthetic parameter.
        detune_cents: 0,
        fx_bus: voice.fx_bus.max(0) as usize,
        activity,
    }
}

fn build_fx_buses(music: &AbcMusic) -> Vec<FxBusOverview> {
    let requested_count = music.fx_bus_count.max(0) as usize;
    let highest_track_bus = music.voices[..music.voice_count.max(0) as usize]
        .iter()
        .map(|voice| voice.fx_bus.max(0) as usize)
        .max()
        .unwrap_or(0);
    let bus_count = usize::max(requested_count, highest_track_bus + 1).clamp(1, ABC_MAX_FX_BUSES);

    music
        .fx_buses
        .iter()
        .take(bus_count)
        .enumerate()
        .map(|(bus_index, bus)| FxBusOverview {
            bus_index,
            enabled: bus.enabled != 0
                || bus.delay_steps > 0
                || bus.drive_amount > 0
                || bus.lowpass_amount > 0
                || bus.sidechain_amount > 0,
            delay_steps: bus.delay_steps,
            delay_feedback: bus.delay_feedback,
            delay_mix: bus.delay_mix,
            drive_amount: bus.drive_amount,
            lowpass_amount: bus.lowpass_amount,
            sidechain_amount: bus.sidechain_amount,
            sidechain_release_ms: if bus.sidechain_release_ms > 0 {
                bus.sidechain_release_ms
            } else {
                ABC_DEFAULT_SIDECHAIN_RELEASE_MS
            },
            mix_percent: if bus.mix_percent > 0 {
                bus.mix_percent
            } else {
                100
            },
        })
        .collect()
}

fn build_instrument_preset_map(music: &AbcMusic) -> HashMap<String, String> {
    let mut presets = HashMap::new();

    for instrument in music
        .instruments
        .iter()
        .take(music.instrument_count.max(0) as usize)
    {
        let name = c_buf_to_string(&instrument.name);
        if !name.is_empty() {
            presets.insert(name, c_buf_to_string(&instrument.preset));
        }
    }

    presets
}

fn build_arrangement(music: &AbcMusic, max_steps: usize) -> Result<Vec<PatternBlock>, String> {
    let mut blocks = Vec::new();
    let mut start_step = 0usize;
    let pattern_count = music.pattern_count.max(0) as usize;
    let arrangement_length = music.arrangement_length.max(0) as usize;

    if pattern_count > 0 && arrangement_length > 0 {
        let lengths = pattern_lengths(music);
        for block_name in music.arrangement.iter().take(arrangement_length) {
            let label = c_buf_to_string(block_name);
            let Some(&length) = lengths.get(&label) else {
                return Err(format!(
                    "arrangement references undefined pattern '{label}'"
                ));
            };
            blocks.push(PatternBlock {
                label,
                length,
                start_step,
            });
            start_step += length;
        }
    } else if pattern_count > 0 {
        for pattern in music.patterns.iter().take(pattern_count) {
            let length = normalized_pattern_length(pattern.length);
            blocks.push(PatternBlock {
                label: c_buf_to_string(&pattern.name),
                length,
                start_step,
            });
            start_step += length;
        }
    } else if max_steps > 0 {
        let mut chunk_index = 0usize;
        while start_step < max_steps {
            let length = usize::min(16, max_steps - start_step);
            blocks.push(PatternBlock {
                label: format!("P{}", chunk_index + 1),
                length,
                start_step,
            });
            start_step += length;
            chunk_index += 1;
        }
    }

    while start_step < max_steps {
        let length = usize::min(16, max_steps - start_step);
        blocks.push(PatternBlock {
            label: format!("P{}", blocks.len() + 1),
            length,
            start_step,
        });
        start_step += length;
    }

    Ok(blocks)
}

fn pattern_lengths(music: &AbcMusic) -> HashMap<String, usize> {
    let mut lengths = HashMap::new();

    for pattern in music
        .patterns
        .iter()
        .take(music.pattern_count.max(0) as usize)
    {
        if pattern.defined == 0 {
            continue;
        }
        lengths.insert(
            c_buf_to_string(&pattern.name),
            normalized_pattern_length(pattern.length),
        );
    }

    lengths
}

fn normalized_pattern_length(length: c_int) -> usize {
    length.clamp(1, 64) as usize
}

fn inferred_steps_per_beat(music: &AbcMusic) -> usize {
    if music.bpm <= 0 || music.step_ms <= 0 {
        return 4;
    }

    ((60_000.0 / music.bpm as f32) / music.step_ms as f32)
        .round()
        .clamp(1.0, 16.0) as usize
}

fn waveform_name(waveform: c_int) -> &'static str {
    match waveform {
        1 => "pulse",
        2 => "triangle",
        3 => "noise",
        _ => "square",
    }
}

fn c_buf_to_string(buffer: &[c_char]) -> String {
    let len = buffer
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(buffer.len());
    let bytes = buffer[..len]
        .iter()
        .map(|&value| value as u8)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).trim().to_string()
}

// ─── Editor song loading ─────────────────────────────────────────────────────

/// Instrument data in a form the editor module can consume.
pub struct RawInstrumentForEditor {
    pub name: String,
    pub preset: String,
    pub waveform: i32,
    pub amplitude: i32,
    pub duty_cycle: i32,
    pub attack_ms: i32,
    pub decay_ms: i32,
    pub sustain_level: i32,
    pub release_ms: i32,
    pub gate_percent: i32,
    pub vibrato_cents: i32,
    pub glide_ms: i32,
    pub fx_bus: i32,
}

/// FX bus data in a form the editor module can consume.
pub struct RawFxBusForEditor {
    pub delay_steps: i32,
    pub delay_feedback: i32,
    pub delay_mix: i32,
    pub drive_amount: i32,
    pub lowpass_amount: i32,
    pub sidechain_amount: i32,
    pub sidechain_release_ms: i32,
    pub mix_percent: i32,
    pub ladder_amount: i32,
    pub ladder_cutoff: i32,
    pub ladder_resonance: i32,
}

/// Pattern definition in a form the editor module can consume.
pub struct RawPatternForEditor {
    pub name: String,
    pub length: usize,
}

/// Voice/track data including the raw note frequencies.
pub struct RawVoiceForEditor {
    pub name: String,
    pub instrument_ref: String,
    pub gate_percent: i32,
    /// Per-step note frequencies in Hz; 0.0 means rest.
    pub note_freqs: Vec<f64>,
}

/// Full song data extracted from `AbcMusic`, consumed by
/// the Rust editor module.
pub struct RawAbcMusicForEditor {
    pub title: String,
    pub bpm: i32,
    pub swing_pct: i32,
    pub instruments: Vec<RawInstrumentForEditor>,
    pub fx_buses: Vec<RawFxBusForEditor>,
    pub patterns: Vec<RawPatternForEditor>,
    pub arrangement: Vec<String>,
    pub voices: Vec<RawVoiceForEditor>,
}

/// Load raw editor song data from an ABC file, preserving per-step note
/// frequencies so the editor model can reconstruct MIDI pitches.
pub fn load_raw_abc_music_for_editor(path: &Path) -> Result<RawAbcMusicForEditor, String> {
    let path_str = normalized_demo_path(path)?;
    let c_path = CString::new(path_str)
        .map_err(|_| format!("demo path contains NUL byte: {}", path.display()))?;
    let mut music = AbcMusic::default();

    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        abc_load(c_path.as_ptr(), &mut music)
    }))
    .map_err(|_| format!("editor load: parser panicked for {}", path.display()))?;
    if result != 0 {
        return Err(format!(
            "editor load: invalid ABC or unreadable file: {}",
            path.display()
        ));
    }

    Ok(extract_raw_abc_music(&music))
}

fn extract_raw_abc_music(music: &AbcMusic) -> RawAbcMusicForEditor {
    let instrument_count = music.instrument_count.max(0) as usize;
    let instruments = music.instruments[..instrument_count]
        .iter()
        .map(|inst| RawInstrumentForEditor {
            name: c_buf_to_string(&inst.name),
            preset: c_buf_to_string(&inst.preset),
            waveform: inst.waveform,
            amplitude: inst.amplitude,
            duty_cycle: inst.duty_cycle,
            attack_ms: inst.attack_ms,
            decay_ms: inst.decay_ms,
            sustain_level: inst.sustain_level,
            release_ms: inst.release_ms,
            gate_percent: inst.gate_percent,
            vibrato_cents: inst.vibrato_cents,
            glide_ms: inst.glide_ms,
            fx_bus: inst.fx_bus,
        })
        .collect();

    let fx_bus_count = {
        let requested = music.fx_bus_count.max(0) as usize;
        let highest = music.voices[..music.voice_count.max(0) as usize]
            .iter()
            .map(|v| v.fx_bus.max(0) as usize)
            .max()
            .unwrap_or(0);
        usize::max(requested, highest + 1).clamp(1, ABC_MAX_FX_BUSES)
    };

    let fx_buses = music.fx_buses[..fx_bus_count]
        .iter()
        .map(|bus| RawFxBusForEditor {
            delay_steps: bus.delay_steps,
            delay_feedback: bus.delay_feedback,
            delay_mix: bus.delay_mix,
            drive_amount: bus.drive_amount,
            lowpass_amount: bus.lowpass_amount,
            sidechain_amount: bus.sidechain_amount,
            sidechain_release_ms: bus.sidechain_release_ms,
            mix_percent: bus.mix_percent,
            ladder_amount: bus.ladder_amount,
            ladder_cutoff: bus.ladder_cutoff,
            ladder_resonance: bus.ladder_resonance,
        })
        .collect();

    let pattern_count = music.pattern_count.max(0) as usize;
    let patterns = music.patterns[..pattern_count]
        .iter()
        .filter(|p| p.defined != 0)
        .map(|p| RawPatternForEditor {
            name: c_buf_to_string(&p.name),
            length: normalized_pattern_length(p.length),
        })
        .collect();

    let arrangement_length = music.arrangement_length.max(0) as usize;
    let arrangement = music.arrangement[..arrangement_length]
        .iter()
        .map(|slot| c_buf_to_string(slot))
        .filter(|s| !s.is_empty())
        .collect();

    let voice_count = music.voice_count.max(0) as usize;
    let voices = music.voices[..voice_count]
        .iter()
        .map(|voice| {
            let note_count = voice.note_count.max(0) as usize;
            RawVoiceForEditor {
                name: c_buf_to_string(&voice.name),
                instrument_ref: c_buf_to_string(&voice.instrument_ref),
                gate_percent: voice.gate_percent,
                note_freqs: voice.freqs[..note_count].iter().map(|&f| f).collect(),
            }
        })
        .collect();

    RawAbcMusicForEditor {
        title: c_buf_to_string(&music.title),
        bpm: music.bpm,
        swing_pct: music.swing_pct,
        instruments,
        fx_buses,
        patterns,
        arrangement,
        voices,
    }
}
