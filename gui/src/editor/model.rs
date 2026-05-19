use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableStep {
    pub active: bool,
    pub midi_note: u8,
    pub velocity: u8,
    pub gate_percent: u8,
    pub accent: bool,
    pub fx_trigger: bool,
}

impl EditableStep {
    pub const DEFAULT_VELOCITY: u8 = 88;
    pub const DEFAULT_GATE_PERCENT: u8 = 90;
    pub const DEFAULT_MIDI_NOTE: u8 = 60;

    pub fn rest() -> Self {
        Self {
            active: false,
            midi_note: 0,
            velocity: Self::DEFAULT_VELOCITY,
            gate_percent: Self::DEFAULT_GATE_PERCENT,
            accent: false,
            fx_trigger: false,
        }
    }

    pub fn note(midi_note: u8) -> Self {
        Self {
            active: midi_note > 0,
            midi_note,
            velocity: Self::DEFAULT_VELOCITY,
            gate_percent: Self::DEFAULT_GATE_PERCENT,
            accent: false,
            fx_trigger: false,
        }
    }

    pub fn toggle_active(&mut self) {
        if self.active {
            *self = Self::rest();
            return;
        }

        if self.midi_note == 0 {
            self.midi_note = Self::DEFAULT_MIDI_NOTE;
        }
        if self.velocity == 0 {
            self.velocity = Self::DEFAULT_VELOCITY;
        }
        if self.gate_percent == 0 {
            self.gate_percent = Self::DEFAULT_GATE_PERCENT;
        }
        self.active = true;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableTrack {
    pub name: String,
    pub instrument_ref: String,
    pub steps: Vec<EditableStep>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditablePattern {
    pub name: String,
    pub length: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableArrangementBlock {
    pub pattern_name: String,
    pub length: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableArrangement {
    pub blocks: Vec<EditableArrangementBlock>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditableInstrument {
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
    pub fx_bus: usize,
}

impl Default for EditableInstrument {
    fn default() -> Self {
        Self {
            name: String::new(),
            preset: String::new(),
            waveform: 0,
            amplitude: 40,
            duty_cycle: 25,
            attack_ms: 0,
            decay_ms: 0,
            sustain_level: 100,
            release_ms: 0,
            gate_percent: 90,
            vibrato_cents: 0,
            glide_ms: 0,
            fx_bus: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditableFxBus {
    pub bus_index: usize,
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

impl EditableFxBus {
    pub fn default_for_index(bus_index: usize) -> Self {
        Self {
            bus_index,
            delay_steps: 0,
            delay_feedback: 0,
            delay_mix: 0,
            drive_amount: 0,
            lowpass_amount: 0,
            sidechain_amount: 0,
            sidechain_release_ms: 180,
            mix_percent: 100,
            ladder_amount: 0,
            ladder_cutoff: 0,
            ladder_resonance: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableSong {
    pub title: String,
    pub tempo: i32,
    pub swing: i32,
    pub patterns: Vec<EditablePattern>,
    pub tracks: Vec<EditableTrack>,
    pub instruments: Vec<EditableInstrument>,
    pub fx_buses: Vec<EditableFxBus>,
    pub arrangement: EditableArrangement,
    pub dirty: bool,
    pub source_path: Option<PathBuf>,
    pub comments: Vec<String>,
}

impl EditableSong {
    pub fn new_song() -> Self {
        let default_pattern = EditablePattern {
            name: "A".to_string(),
            length: 16,
        };
        let default_instrument = EditableInstrument {
            name: "lead".to_string(),
            ..Default::default()
        };

        Self {
            title: "Untitled".to_string(),
            tempo: 120,
            swing: 50,
            patterns: vec![default_pattern.clone()],
            tracks: vec![EditableTrack {
                name: "track_01".to_string(),
                instrument_ref: default_instrument.name.clone(),
                steps: vec![EditableStep::rest(); default_pattern.length],
            }],
            instruments: vec![default_instrument],
            fx_buses: vec![EditableFxBus::default_for_index(0)],
            arrangement: EditableArrangement {
                blocks: vec![EditableArrangementBlock {
                    pattern_name: "A".to_string(),
                    length: 16,
                }],
            },
            dirty: false,
            source_path: None,
            comments: Vec::new(),
        }
    }

    pub fn total_steps(&self) -> usize {
        self.arrangement.blocks.iter().map(|b| b.length).sum::<usize>().max(1)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean_with_path(&mut self, path: Option<PathBuf>) {
        self.dirty = false;
        if path.is_some() {
            self.source_path = path;
        }
    }
}
