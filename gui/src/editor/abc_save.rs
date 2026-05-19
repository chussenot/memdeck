use std::fs;
use std::path::Path;

use super::model::{EditableArrangementBlock, EditableFxBus, EditableInstrument, EditableSong};
use super::validation::validate_song;

pub fn save_editable_song_to_path(song: &mut EditableSong, path: &Path) -> Result<(), String> {
    let abc = serialize_editable_song(song)?;
    fs::write(path, abc).map_err(|error| format!("save failed for {}: {error}", path.display()))?;
    song.mark_clean_with_path(Some(path.to_path_buf()));
    Ok(())
}

pub fn serialize_editable_song(song: &EditableSong) -> Result<String, String> {
    validate_song(song)?;

    let mut out = String::with_capacity(4096);
    out.push_str("X:1\n");
    out.push_str(&format!("T:{}\n", song.title));
    out.push_str("C:MemDeck\n");
    out.push_str("M:4/4\n");
    out.push_str("L:1/16\n");
    out.push_str(&format!("Q:1/4={}\n", song.tempo));
    out.push_str("K:C\n");

    if !song.comments.is_empty() {
        for comment in &song.comments {
            out.push_str(comment.trim());
            out.push('\n');
        }
    }

    out.push_str(&format!("%%swing {}\n", song.swing));

    for instrument in &song.instruments {
        out.push_str(&format_instrument_directive(instrument));
    }

    for bus in &song.fx_buses {
        out.push_str(&format_effect_directive(bus));
    }

    for pattern in &song.patterns {
        out.push_str(&format!("%%pattern {} length={}\n", pattern.name, pattern.length));
    }

    let arrangement_labels = song
        .arrangement
        .blocks
        .iter()
        .map(|b| b.pattern_name.as_str())
        .collect::<Vec<_>>();
    out.push_str(&format!("%%arrangement {}\n", arrangement_labels.join(" ")));

    for track in &song.tracks {
        if track.instrument_ref.is_empty() {
            out.push_str(&format!("V:{}\n", track.name));
        } else {
            out.push_str(&format!("V:{} instrument={}\n", track.name, track.instrument_ref));
        }
    }

    for track in &song.tracks {
        out.push_str(&format!("V:{}\n", track.name));
        out.push_str(&format_track_steps(
            track.steps.as_slice(),
            song.arrangement.blocks.as_slice(),
        ));
    }

    for (track_index, track) in song.tracks.iter().enumerate() {
        out.push_str(&format_step_directives(track_index, track.steps.as_slice()));
    }

    Ok(out)
}

fn format_instrument_directive(inst: &EditableInstrument) -> String {
    let wave = waveform_id_to_name(inst.waveform);
    let mut line = format!(
        "%%instrument {} wave={} amp={} duty={} attack={} decay={} sustain={} release={} gate={}",
        inst.name,
        wave,
        inst.amplitude,
        inst.duty_cycle,
        inst.attack_ms,
        inst.decay_ms,
        inst.sustain_level,
        inst.release_ms,
        inst.gate_percent,
    );

    if inst.vibrato_cents > 0 {
        line.push_str(&format!(" vibrato={}", inst.vibrato_cents));
    }
    if inst.glide_ms > 0 {
        line.push_str(&format!(" glide={}", inst.glide_ms));
    }
    if !inst.preset.is_empty() {
        line.push_str(&format!(" preset={}", inst.preset));
    }
    line.push_str(&format!(" fx={}\n", inst.fx_bus));
    line
}

fn format_effect_directive(bus: &EditableFxBus) -> String {
    let mut line = format!(
        "%%effect {} delay_steps={} delay_feedback={} delay_mix={} drive={} lowpass={} sidechain={} sidechain_release={} mix={}",
        bus.bus_index,
        bus.delay_steps,
        bus.delay_feedback,
        bus.delay_mix,
        bus.drive_amount,
        bus.lowpass_amount,
        bus.sidechain_amount,
        bus.sidechain_release_ms,
        bus.mix_percent,
    );
    if bus.ladder_amount > 0 {
        line.push_str(&format!(
            " ladder={} ladder_cutoff={} ladder_resonance={}",
            bus.ladder_amount, bus.ladder_cutoff, bus.ladder_resonance
        ));
    }
    line.push('\n');
    line
}

fn format_track_steps(steps: &[super::model::EditableStep], blocks: &[EditableArrangementBlock]) -> String {
    if steps.is_empty() {
        return "| z16 |\n".to_string();
    }

    let mut out = String::new();
    let mut step_cursor = 0usize;
    for block in blocks {
        out.push_str(&format!("% section {}\n", block.pattern_name));
        out.push_str("| ");
        let end = (step_cursor + block.length).min(steps.len());
        for step in &steps[step_cursor..end] {
            if step.active && step.midi_note > 0 {
                out.push_str(&midi_to_abc_note(step.midi_note));
            } else {
                out.push('z');
            }
        }
        for _ in (end - step_cursor)..block.length {
            out.push('z');
        }
        out.push_str(" |\n");
        step_cursor += block.length;
    }

    if step_cursor < steps.len() {
        out.push_str("| ");
        for step in &steps[step_cursor..] {
            if step.active && step.midi_note > 0 {
                out.push_str(&midi_to_abc_note(step.midi_note));
            } else {
                out.push('z');
            }
        }
        out.push_str(" |\n");
    }

    out
}

fn format_step_directives(track_index: usize, steps: &[super::model::EditableStep]) -> String {
    let mut out = String::new();
    for (step_index, step) in steps.iter().enumerate() {
        let has_custom_velocity = step.velocity != super::model::EditableStep::DEFAULT_VELOCITY;
        let has_custom_gate = step.gate_percent != super::model::EditableStep::DEFAULT_GATE_PERCENT;
        if !has_custom_velocity && !has_custom_gate && !step.accent && !step.fx_trigger {
            continue;
        }
        out.push_str(&format!(
            "%%mdstep t={} s={} vel={} gate={} accent={} fx={}\n",
            track_index,
            step_index,
            step.velocity.clamp(1, 127),
            step.gate_percent.clamp(1, 100),
            if step.accent { 1 } else { 0 },
            if step.fx_trigger { 1 } else { 0 },
        ));
    }
    out
}

fn midi_to_abc_note(midi: u8) -> String {
    const SHARP_NAMES_UPPER: [&str; 12] = [
        "C", "^C", "D", "^D", "E", "F", "^F", "G", "^G", "A", "^A", "B",
    ];
    const SHARP_NAMES_LOWER: [&str; 12] = [
        "c", "^c", "d", "^d", "e", "f", "^f", "g", "^g", "a", "^a", "b",
    ];

    if midi == 0 {
        return "z".to_string();
    }

    let semitone = (midi % 12) as usize;
    let octave = (midi / 12) as i32 - 1;

    match octave {
        4 => SHARP_NAMES_LOWER[semitone].to_string(),
        5 => format!("{}'", SHARP_NAMES_LOWER[semitone]),
        6 => format!("{}''", SHARP_NAMES_LOWER[semitone]),
        7 => format!("{}'''", SHARP_NAMES_LOWER[semitone]),
        3 => SHARP_NAMES_UPPER[semitone].to_string(),
        2 => format!("{},", SHARP_NAMES_UPPER[semitone]),
        1 => format!("{},,", SHARP_NAMES_UPPER[semitone]),
        _ => "c".to_string(),
    }
}

fn waveform_id_to_name(id: i32) -> &'static str {
    match id {
        1 => "pulse",
        2 => "triangle",
        3 => "noise",
        _ => "square",
    }
}
