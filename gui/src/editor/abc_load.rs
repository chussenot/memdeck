use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ffi;

use super::model::{
    EditableArrangement, EditableArrangementBlock, EditableFxBus, EditableInstrument,
    EditablePattern, EditableSong, EditableStep, EditableTrack,
};
use super::validation::validate_song;

pub fn load_editable_song_from_path(path: &Path) -> Result<EditableSong, String> {
    let (comments, step_directives) = load_comments_and_step_directives(path);
    let raw = ffi::load_raw_abc_music_for_editor(path)?;
    let mut song = build_editable_song_from_raw(raw);
    apply_step_directives(&mut song, &step_directives);
    song.comments = comments;
    song.source_path = Some(path.to_path_buf());
    song.dirty = false;
    validate_song(&song)?;
    Ok(song)
}

#[derive(Clone, Debug, Default)]
struct StepDirective {
    track: usize,
    step: usize,
    velocity: Option<u8>,
    gate_percent: Option<u8>,
    accent: Option<bool>,
    fx_trigger: Option<bool>,
}

fn load_comments_and_step_directives(path: &Path) -> (Vec<String>, Vec<StepDirective>) {
    let Ok(source) = fs::read_to_string(path) else {
        return (Vec::new(), Vec::new());
    };

    let mut comments = Vec::new();
    let mut directives = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if let Some(directive) = parse_step_directive(line) {
            directives.push(directive);
            continue;
        }
        if line.starts_with('%') && !line.starts_with("%%") {
            comments.push(line.to_string());
        }
    }
    (comments, directives)
}

pub fn build_editable_song_from_raw(raw: ffi::RawAbcMusicForEditor) -> EditableSong {
    let instruments = raw
        .instruments
        .into_iter()
        .map(|inst| EditableInstrument {
            name: inst.name,
            preset: inst.preset,
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
            fx_bus: inst.fx_bus as usize,
        })
        .collect::<Vec<_>>();

    let mut fx_buses = raw
        .fx_buses
        .iter()
        .enumerate()
        .map(|(i, bus)| EditableFxBus {
            bus_index: i,
            delay_steps: bus.delay_steps,
            delay_feedback: bus.delay_feedback,
            delay_mix: bus.delay_mix,
            drive_amount: bus.drive_amount,
            lowpass_amount: bus.lowpass_amount,
            sidechain_amount: bus.sidechain_amount,
            sidechain_release_ms: if bus.sidechain_release_ms > 0 {
                bus.sidechain_release_ms
            } else {
                180
            },
            mix_percent: if bus.mix_percent > 0 { bus.mix_percent } else { 100 },
            ladder_amount: bus.ladder_amount,
            ladder_cutoff: bus.ladder_cutoff,
            ladder_resonance: bus.ladder_resonance,
        })
        .collect::<Vec<_>>();
    if fx_buses.is_empty() {
        fx_buses.push(EditableFxBus::default_for_index(0));
    }

    let patterns = raw
        .patterns
        .into_iter()
        .map(|p| EditablePattern {
            name: p.name,
            length: p.length.max(1),
        })
        .collect::<Vec<_>>();

    let pattern_lengths = patterns
        .iter()
        .map(|pattern| (pattern.name.clone(), pattern.length))
        .collect::<HashMap<_, _>>();

    let arrangement = if raw.arrangement.is_empty() {
        EditableArrangement {
            blocks: patterns
                .iter()
                .map(|pattern| EditableArrangementBlock {
                    pattern_name: pattern.name.clone(),
                    length: pattern.length,
                })
                .collect(),
        }
    } else {
        EditableArrangement {
            blocks: raw
                .arrangement
                .into_iter()
                .map(|name| EditableArrangementBlock {
                    length: pattern_lengths.get(&name).copied().unwrap_or(16),
                    pattern_name: name,
                })
                .collect(),
        }
    };

    let tracks = raw
        .voices
        .into_iter()
                .map(|voice| EditableTrack {
                    name: voice.name,
                    instrument_ref: voice.instrument_ref,
                    steps: voice
                        .note_freqs
                .iter()
                        .map(|&freq| {
                            if freq <= 0.0 {
                                EditableStep::rest()
                            } else {
                                let mut step = EditableStep::note(freq_to_midi(freq));
                                step.gate_percent = voice.gate_percent.clamp(1, 100) as u8;
                                step
                            }
                        })
                        .collect(),
                })
        .collect::<Vec<_>>();

    EditableSong {
        title: raw.title,
        tempo: raw.bpm,
        swing: if raw.swing_pct > 0 { raw.swing_pct } else { 50 },
        patterns,
        tracks,
        instruments,
        fx_buses,
        arrangement,
        dirty: false,
        source_path: None,
        comments: Vec::new(),
    }
}

fn freq_to_midi(freq: f64) -> u8 {
    if freq <= 0.0 {
        return 0;
    }
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    midi.round().clamp(1.0, 127.0) as u8
}

fn parse_step_directive(line: &str) -> Option<StepDirective> {
    const PREFIX: &str = "%%mdstep";
    if !line.starts_with(PREFIX) {
        return None;
    }
    let mut directive = StepDirective::default();
    let mut has_track = false;
    let mut has_step = false;

    for token in line[PREFIX.len()..].split_whitespace() {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        match key {
            "t" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    directive.track = parsed;
                    has_track = true;
                }
            }
            "s" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    directive.step = parsed;
                    has_step = true;
                }
            }
            "vel" => directive.velocity = value.parse::<u8>().ok(),
            "gate" => directive.gate_percent = value.parse::<u8>().ok(),
            "accent" => directive.accent = parse_bool(value),
            "fx" => directive.fx_trigger = parse_bool(value),
            _ => {}
        }
    }

    if has_track && has_step {
        Some(directive)
    } else {
        None
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "true" | "TRUE" => Some(true),
        "0" | "false" | "FALSE" => Some(false),
        _ => None,
    }
}

fn apply_step_directives(song: &mut EditableSong, directives: &[StepDirective]) {
    for directive in directives {
        let Some(track) = song.tracks.get_mut(directive.track) else {
            continue;
        };
        let Some(step) = track.steps.get_mut(directive.step) else {
            continue;
        };

        if let Some(velocity) = directive.velocity {
            step.velocity = velocity.clamp(1, 127);
        }
        if let Some(gate_percent) = directive.gate_percent {
            step.gate_percent = gate_percent.clamp(1, 100);
        }
        if let Some(accent) = directive.accent {
            step.accent = accent;
        }
        if let Some(fx_trigger) = directive.fx_trigger {
            step.fx_trigger = fx_trigger;
        }
    }
}
