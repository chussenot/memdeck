use std::collections::HashSet;

use super::model::EditableSong;

pub fn validate_song(song: &EditableSong) -> Result<(), String> {
    if song.title.trim().is_empty() {
        return Err("song title cannot be empty".to_string());
    }
    if song.tempo <= 0 {
        return Err("tempo must be positive".to_string());
    }
    if !(0..=100).contains(&song.swing) {
        return Err("swing must be between 0 and 100".to_string());
    }
    if song.tracks.is_empty() {
        return Err("song must contain at least one track".to_string());
    }
    if song.instruments.is_empty() {
        return Err("song must contain at least one instrument".to_string());
    }
    if song.fx_buses.is_empty() {
        return Err("song must contain at least one FX bus".to_string());
    }
    if song.patterns.is_empty() {
        return Err("song must contain at least one pattern".to_string());
    }
    if song.arrangement.blocks.is_empty() {
        return Err("song must contain at least one arrangement block".to_string());
    }

    let mut pattern_names = HashSet::new();
    for pattern in &song.patterns {
        if pattern.name.trim().is_empty() {
            return Err("pattern name cannot be empty".to_string());
        }
        if pattern.length == 0 {
            return Err(format!("pattern '{}' must have a positive length", pattern.name));
        }
        if !pattern_names.insert(pattern.name.clone()) {
            return Err(format!("duplicate pattern name '{}'", pattern.name));
        }
    }

    for block in &song.arrangement.blocks {
        if block.length == 0 {
            return Err("arrangement block length must be positive".to_string());
        }
        if !pattern_names.contains(&block.pattern_name) {
            return Err(format!(
                "arrangement references undefined pattern '{}'",
                block.pattern_name
            ));
        }
    }

    for track in &song.tracks {
        if track.name.trim().is_empty() {
            return Err("track name cannot be empty".to_string());
        }
        for (step_index, step) in track.steps.iter().enumerate() {
            if !(1..=127).contains(&step.velocity) {
                return Err(format!(
                    "track '{}' step {} has invalid velocity {}; expected 1..=127",
                    track.name,
                    step_index,
                    step.velocity
                ));
            }
            if !(1..=100).contains(&step.gate_percent) {
                return Err(format!(
                    "track '{}' step {} has invalid gate {}; expected 1..=100",
                    track.name,
                    step_index,
                    step.gate_percent
                ));
            }
            if step.active && step.midi_note == 0 {
                return Err(format!(
                    "track '{}' step {} is active but has no note",
                    track.name, step_index
                ));
            }
            if step.midi_note > 127 {
                return Err(format!(
                    "track '{}' step {} has midi note out of range",
                    track.name, step_index
                ));
            }
        }
    }

    Ok(())
}
