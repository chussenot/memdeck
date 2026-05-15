use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::path::Path;
use std::slice;
use std::sync::{LazyLock, Mutex};

use crate::audio_engine::{DemoOverview, PatternBlock, TrackOverview};

pub const SAMPLE_RATE_ABC: c_int = 22_050;
const ABC_MAX_VOICES: usize = 8;
const ABC_MAX_NOTES: usize = 1024;
const ABC_MAX_INSTRUMENTS: usize = 8;
const ABC_MAX_PATTERNS: usize = 8;
const ABC_MAX_ARRANGEMENT: usize = 32;
const ABC_MAX_FX_BUSES: usize = 4;
const SEQ_MAX_TRACKS: usize = 4;

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
}

static LAST_RENDER_STATS: LazyLock<Mutex<Option<AudioRenderStats>>> =
    LazyLock::new(|| Mutex::new(None));

fn copy_and_release_buffer(buffer: *mut u8, len: c_int) -> Result<Vec<u8>, String> {
    if buffer.is_null() || len <= 0 {
        return Err("audio render failed or returned empty PCM buffer".to_string());
    }

    let samples = unsafe {
        let slice = slice::from_raw_parts(buffer as *const u8, len as usize);
        slice.to_vec()
    };

    free_buffer(buffer);
    Ok(samples)
}

pub fn render_abc_file(path: &Path) -> Result<Vec<u8>, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "demo path is not valid UTF-8".to_string())?;
    let c_path = CString::new(path_str).map_err(|_| "demo path contains NUL byte".to_string())?;

    let mut pcm_len = 0;
    let mut stats = AudioRenderStats::default();

    let buffer = unsafe {
        audio_engine_render_abc_file(c_path.as_ptr(), SAMPLE_RATE_ABC, &mut pcm_len, &mut stats)
    };
    let samples = copy_and_release_buffer(buffer, pcm_len)?;

    if let Ok(mut last_stats) = LAST_RENDER_STATS.lock() {
        *last_stats = Some(stats);
    }

    Ok(samples)
}

pub fn load_demo_overview(path: &Path) -> Result<DemoOverview, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "demo path is not valid UTF-8".to_string())?;
    let c_path = CString::new(path_str).map_err(|_| "demo path contains NUL byte".to_string())?;
    let mut music = AbcMusic::default();

    let result = unsafe { abc_load(c_path.as_ptr(), &mut music) };
    if result != 0 {
        return Err(format!("invalid ABC or unreadable demo: {}", path.display()));
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
    let instrument_presets = build_instrument_preset_map(music);
    let visible_tracks = usize::min(music.voice_count.max(0) as usize, SEQ_MAX_TRACKS);
    let hidden_track_count = music.voice_count.max(0) as usize - visible_tracks;
    let max_steps = music.voices[..visible_tracks]
        .iter()
        .map(|voice| voice.note_count.max(0) as usize)
        .max()
        .unwrap_or(0);

    let arrangement = build_arrangement(music, max_steps)?;
    let total_steps = arrangement
        .last()
        .map(|block| block.start_step + block.length)
        .unwrap_or(max_steps);

    let tracks = music.voices[..visible_tracks]
        .iter()
        .enumerate()
        .map(|(index, voice)| {
            let name = c_buf_to_string(&voice.name);
            let instrument_ref = c_buf_to_string(&voice.instrument_ref);
            let display_name = if name.is_empty() {
                format!("track_{:02}", index + 1)
            } else {
                name
            };
            let instrument = if instrument_ref.is_empty() {
                "direct".to_string()
            } else if let Some(preset) = instrument_presets.get(&instrument_ref) {
                format!("{instrument_ref} · {preset}")
            } else {
                instrument_ref
            };

            let mut activity = vec![false; total_steps.max(max_steps)];
            for (step_index, &frequency) in voice
                .freqs
                .iter()
                .take(voice.note_count.max(0) as usize)
                .enumerate()
            {
                if step_index < activity.len() && frequency > 0.0 {
                    activity[step_index] = true;
                }
            }

            TrackOverview {
                name: display_name,
                instrument,
                activity,
            }
        })
        .collect();

    Ok(DemoOverview {
        title: c_buf_to_string(&music.title),
        bpm: music.bpm,
        swing_pct: if music.swing_pct <= 0 { 50 } else { music.swing_pct },
        total_steps: total_steps.max(max_steps),
        arrangement,
        tracks,
        hidden_track_count,
    })
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
            let preset = c_buf_to_string(&instrument.preset);
            presets.insert(name, preset);
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
                return Err(format!("arrangement references undefined pattern '{label}'"));
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

    for pattern in music.patterns.iter().take(music.pattern_count.max(0) as usize) {
        lengths.insert(c_buf_to_string(&pattern.name), normalized_pattern_length(pattern.length));
    }

    lengths
}

fn normalized_pattern_length(length: c_int) -> usize {
    length.clamp(1, 64) as usize
}

fn c_buf_to_string(buffer: &[c_char]) -> String {
    let len = buffer.iter().position(|&value| value == 0).unwrap_or(buffer.len());
    let bytes = buffer[..len]
        .iter()
        .map(|&value| value as u8)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).trim().to_string()
}
