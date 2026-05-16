//! MemDeck Song Editor Model
//!
//! Provides the in-memory editable representation of a MemDeck song, distinct from the
//! read-only `DemoOverview` used by the browser panels. The editor model can be loaded
//! from an ABC file, mutated in memory, and serialised back to the MemDeck ABC DSL so
//! that the existing C audio engine can render it deterministically.
//!
//! ## Design principles
//! - No DAW concepts (clips, audio regions, plug-in chains).
//! - Keyboard-first, step-sequencer-style editing inspired by Atari ST sequencers.
//! - The read-only browser/render mode is never disturbed; the editor model is a
//!   parallel data path that coexists with the existing `GuiAudioEngine`.
//!
//! ## Editor modes
//! - [`EditorMode::Browser`]  – existing read-only demo browser (default on startup).
//! - [`EditorMode::Edit`]     – in-memory editable song is active.
//! - [`EditorMode::PreviewRender`] – song is exported to ABC DSL and re-rendered through
//!   the C engine for playback preview.

use std::path::Path;

use crate::ffi;

// ─── Editor mode ─────────────────────────────────────────────────────────────

/// Application-level editor mode.
///
/// Only `Browser` is wired into the GUI today; `Edit` and `PreviewRender` are
/// declared so the architecture is in place for future incremental work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorMode {
    /// Read-only demo browser — the current default experience.
    #[default]
    Browser,
    /// Active in-memory song editor.
    Edit,
    /// ABC DSL has been exported; the C engine is rendering a preview.
    PreviewRender,
}

impl EditorMode {
    pub fn label(self) -> &'static str {
        match self {
            EditorMode::Browser => "BROWSER",
            EditorMode::Edit => "EDIT",
            EditorMode::PreviewRender => "PREVIEW",
        }
    }
}

// ─── Editable leaf types ─────────────────────────────────────────────────────

/// One time-step in a track's note sequence.
///
/// `midi_note == 0` is a rest (silence); values 1–127 are standard MIDI pitches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditableStep {
    /// Whether this step produces a sound (false when `midi_note == 0`).
    pub active: bool,
    /// MIDI note number (0 = rest, 1–127 = pitched).
    pub midi_note: u8,
}

impl EditableStep {
    pub fn rest() -> Self {
        Self {
            active: false,
            midi_note: 0,
        }
    }

    pub fn note(midi: u8) -> Self {
        Self {
            active: midi > 0,
            midi_note: midi,
        }
    }
}

/// One voice/track in the editable song.
#[derive(Clone, Debug, Default)]
pub struct EditableTrack {
    pub name: String,
    /// References an [`EditableInstrument`] by name.
    pub instrument_ref: String,
    pub steps: Vec<EditableStep>,
}

/// A named pattern region with a step count.
///
/// Patterns label sections of the linear note stream; they do not store note
/// data themselves (notes live in [`EditableTrack::steps`]).
#[derive(Clone, Debug, Default)]
pub struct EditablePattern {
    pub name: String,
    pub length: usize,
}

/// Instrument / synth-voice definition.
///
/// Mirrors `AbcInstrument` from the FFI layer without the C fixed-size buffers.
#[derive(Clone, Debug)]
pub struct EditableInstrument {
    pub name: String,
    pub preset: String,
    /// `0` = square, `1` = pulse, `2` = triangle, `3` = noise.
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
    /// Index into [`EditableSong::fx_buses`].
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

/// One FX bus in the signal chain.
#[derive(Clone, Debug)]
pub struct EditableFxBus {
    /// Zero-based bus index.
    pub bus_index: usize,
    pub delay_steps: i32,
    pub delay_feedback: i32,
    pub delay_mix: i32,
    pub drive_amount: i32,
    pub lowpass_amount: i32,
    pub sidechain_amount: i32,
    pub sidechain_release_ms: i32,
    pub mix_percent: i32,
}

impl EditableFxBus {
    fn default_for_index(bus_index: usize) -> Self {
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
        }
    }
}

// ─── EditableSong ─────────────────────────────────────────────────────────────

/// The top-level editable song model.
///
/// Load with [`EditableSong::load_from_path`], mutate fields in memory, then
/// call [`EditableSong::to_abc_dsl`] to export back to a string that the C
/// engine can parse and render deterministically.
#[derive(Clone, Debug, Default)]
pub struct EditableSong {
    pub title: String,
    pub bpm: i32,
    pub swing_pct: i32,
    /// Named pattern regions (label + step count).
    pub patterns: Vec<EditablePattern>,
    /// Ordered pattern names forming the arrangement timeline.
    pub arrangement: Vec<String>,
    pub instruments: Vec<EditableInstrument>,
    pub fx_buses: Vec<EditableFxBus>,
    pub tracks: Vec<EditableTrack>,
}

impl EditableSong {
    /// Load an `EditableSong` from an ABC file on disk.
    ///
    /// Delegates to the existing FFI `abc_load` path and preserves the full
    /// per-step note-frequency data (unlike the read-only `DemoOverview` which
    /// only stores step-activity booleans).
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        ffi::load_editor_song(path)
    }

    /// Serialise the song to the MemDeck ABC DSL.
    ///
    /// The resulting string is accepted by the C `abc_load` / render pipeline.
    /// A simple roundtrip (load → serialise → re-parse) preserves tempo, swing,
    /// instruments, FX buses, patterns, arrangement, and per-step pitches.
    pub fn to_abc_dsl(&self) -> String {
        let mut out = String::with_capacity(4096);

        // ── Standard ABC header ───────────────────────────────────────────
        out.push_str("X:1\n");
        out.push_str(&format!("T:{}\n", self.title));
        out.push_str("C:MemDeck\n");
        out.push_str("M:4/4\n");
        out.push_str("L:1/16\n");
        out.push_str(&format!("Q:1/4={}\n", self.bpm));
        out.push_str("K:C\n");

        // ── MemDeck DSL directives ────────────────────────────────────────
        out.push_str(&format!("%%swing {}\n", self.swing_pct));

        for inst in &self.instruments {
            out.push_str(&self.format_instrument_directive(inst));
        }

        for bus in &self.fx_buses {
            out.push_str(&format_effect_directive(bus));
        }

        for pattern in &self.patterns {
            out.push_str(&format!("%%pattern {} length={}\n", pattern.name, pattern.length));
        }

        if !self.arrangement.is_empty() {
            out.push_str(&format!("%%arrangement {}\n", self.arrangement.join(" ")));
        }

        // ── Voice declarations ────────────────────────────────────────────
        for track in &self.tracks {
            if track.instrument_ref.is_empty() {
                out.push_str(&format!("V:{}\n", track.name));
            } else {
                out.push_str(&format!(
                    "V:{} instrument={}\n",
                    track.name, track.instrument_ref
                ));
            }
        }

        // ── Voice note sections ───────────────────────────────────────────
        for track in &self.tracks {
            out.push_str(&format!("V:{}\n", track.name));
            out.push_str(&format_voice_steps(&track.steps, &self.patterns, &self.arrangement));
        }

        out
    }

    fn format_instrument_directive(&self, inst: &EditableInstrument) -> String {
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
}

// ─── ABC formatting helpers ──────────────────────────────────────────────────

fn format_effect_directive(bus: &EditableFxBus) -> String {
    format!(
        "%%effect {} delay_steps={} delay_feedback={} delay_mix={} drive={} lowpass={} sidechain={} sidechain_release={} mix={}\n",
        bus.bus_index,
        bus.delay_steps,
        bus.delay_feedback,
        bus.delay_mix,
        bus.drive_amount,
        bus.lowpass_amount,
        bus.sidechain_amount,
        bus.sidechain_release_ms,
        bus.mix_percent,
    )
}

/// Format a track's steps as one or more bar lines.
///
/// Steps are emitted as individual 1/16-note symbols (`c`, `^c`, `z`, etc.)
/// grouped into bars based on the pattern length (or 16 by default).
fn format_voice_steps(
    steps: &[EditableStep],
    patterns: &[EditablePattern],
    arrangement: &[String],
) -> String {
    if steps.is_empty() {
        return "| z16 |\n".to_string();
    }

    // Build an ordered list of (section_label, bar_length) from arrangement or
    // patterns; fall back to fixed 16-step bars if neither is available.
    let bar_boundaries = compute_bar_boundaries(steps.len(), patterns, arrangement);

    let mut out = String::new();
    let mut step_cursor = 0usize;

    for (label, length) in &bar_boundaries {
        out.push_str(&format!("% section {label}\n"));
        let end = (step_cursor + length).min(steps.len());
        let bar_steps = &steps[step_cursor..end];
        out.push_str("| ");
        for step in bar_steps {
            if step.active && step.midi_note > 0 {
                out.push_str(&midi_to_abc_note(step.midi_note));
            } else {
                out.push('z');
            }
        }
        // Pad to declared length with rests if the track is shorter.
        for _ in (end - step_cursor)..*length {
            out.push('z');
        }
        out.push_str(" |\n");
        step_cursor += length;
    }

    // Emit any remaining steps that fall outside declared patterns.
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

/// Return `(section_name, step_count)` pairs that cover the full arrangement.
fn compute_bar_boundaries(
    total_steps: usize,
    patterns: &[EditablePattern],
    arrangement: &[String],
) -> Vec<(String, usize)> {
    let mut result = Vec::new();

    if !patterns.is_empty() && !arrangement.is_empty() {
        // Use named arrangement order.
        let lengths: std::collections::HashMap<&str, usize> = patterns
            .iter()
            .map(|p| (p.name.as_str(), p.length))
            .collect();
        let mut covered = 0usize;
        for name in arrangement {
            let length = *lengths.get(name.as_str()).unwrap_or(&16);
            if covered >= total_steps {
                break;
            }
            result.push((name.clone(), length));
            covered += length;
        }
    } else if !patterns.is_empty() {
        // No explicit arrangement — emit patterns in definition order.
        let mut covered = 0usize;
        for pattern in patterns {
            if covered >= total_steps {
                break;
            }
            result.push((pattern.name.clone(), pattern.length));
            covered += pattern.length;
        }
    } else {
        // No pattern metadata — chunk into 16-step bars.
        let mut start = 0usize;
        let mut idx = 1usize;
        while start < total_steps {
            let length = 16.min(total_steps - start);
            result.push((format!("P{idx}"), length));
            start += length;
            idx += 1;
        }
    }

    result
}

/// Convert a MIDI note number (1–127) to its ABC note-name string.
///
/// Mapping follows standard ABC: `c` = middle C (C4, MIDI 60).
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
    // MIDI 60 = C4, (60 / 12) - 1 = 4
    let octave = (midi / 12) as i32 - 1;

    match octave {
        4 => SHARP_NAMES_LOWER[semitone].to_string(),
        5 => format!("{}'", SHARP_NAMES_LOWER[semitone]),
        6 => format!("{}''", SHARP_NAMES_LOWER[semitone]),
        7 => format!("{}'''", SHARP_NAMES_LOWER[semitone]),
        3 => SHARP_NAMES_UPPER[semitone].to_string(),
        2 => format!("{},", SHARP_NAMES_UPPER[semitone]),
        1 => format!("{},,", SHARP_NAMES_UPPER[semitone]),
        0 => format!("{},,,", SHARP_NAMES_UPPER[semitone]),
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

// ─── FFI bridge helper (lives here to avoid circular imports) ────────────────

/// Build an [`EditableSong`] from the raw `AbcMusic` data exposed by `ffi`.
///
/// This function is called by [`ffi::load_editor_song`] once the C parser has
/// populated the `AbcMusic` struct; it converts C-compatible fixed-size arrays
/// into owned Rust types suitable for in-memory editing.
pub(crate) fn build_editable_song_from_ffi(raw: ffi::RawAbcMusicForEditor) -> EditableSong {
    // ── Instruments ───────────────────────────────────────────────────────
    let instruments: Vec<EditableInstrument> = raw
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
        .collect();

    // ── FX buses ─────────────────────────────────────────────────────────
    let fx_buses: Vec<EditableFxBus> = raw
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
            mix_percent: if bus.mix_percent > 0 {
                bus.mix_percent
            } else {
                100
            },
        })
        .collect();

    // Ensure at least one bus exists so every track can route to bus 0.
    let fx_buses = if fx_buses.is_empty() {
        vec![EditableFxBus::default_for_index(0)]
    } else {
        fx_buses
    };

    // ── Patterns ─────────────────────────────────────────────────────────
    let patterns: Vec<EditablePattern> = raw
        .patterns
        .into_iter()
        .map(|p| EditablePattern {
            name: p.name,
            length: p.length,
        })
        .collect();

    // ── Tracks + steps ────────────────────────────────────────────────────
    let tracks: Vec<EditableTrack> = raw
        .voices
        .into_iter()
        .map(|voice| {
            let steps = voice
                .note_freqs
                .iter()
                .map(|&freq| {
                    if freq <= 0.0 {
                        EditableStep::rest()
                    } else {
                        EditableStep::note(freq_to_midi(freq))
                    }
                })
                .collect();
            EditableTrack {
                name: voice.name,
                instrument_ref: voice.instrument_ref,
                steps,
            }
        })
        .collect();

    EditableSong {
        title: raw.title,
        bpm: raw.bpm,
        swing_pct: if raw.swing_pct > 0 { raw.swing_pct } else { 50 },
        patterns,
        arrangement: raw.arrangement,
        instruments,
        fx_buses,
        tracks,
    }
}

/// Convert a frequency in Hz to the nearest MIDI note number (1–127).
///
/// Uses the same A4=440 equal-temperament formula as the C audio engine so
/// that round-tripping via ABC note names preserves the original pitches.
fn freq_to_midi(freq: f64) -> u8 {
    if freq <= 0.0 {
        return 0;
    }
    // MIDI note = 69 + 12 * log2(freq / 440)
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    midi.round().clamp(1.0, 127.0) as u8
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
    }

    fn demo_path(name: &str) -> PathBuf {
        repo_root().join("data").join("music").join(format!("{name}.abc"))
    }

    #[test]
    fn editor_mode_labels_are_unique() {
        let labels: Vec<_> = [EditorMode::Browser, EditorMode::Edit, EditorMode::PreviewRender]
            .iter()
            .map(|m| m.label())
            .collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len(), "every mode must have a unique label");
    }

    #[test]
    fn editable_step_rest_is_inactive() {
        let step = EditableStep::rest();
        assert!(!step.active);
        assert_eq!(step.midi_note, 0);
    }

    #[test]
    fn editable_step_note_is_active() {
        let step = EditableStep::note(60);
        assert!(step.active);
        assert_eq!(step.midi_note, 60);
    }

    #[test]
    fn midi_to_abc_middle_c() {
        assert_eq!(midi_to_abc_note(60), "c");
    }

    #[test]
    fn midi_to_abc_a4() {
        assert_eq!(midi_to_abc_note(69), "a");
    }

    #[test]
    fn midi_to_abc_c3_uppercase() {
        assert_eq!(midi_to_abc_note(48), "C");
    }

    #[test]
    fn midi_to_abc_c5_tick() {
        assert_eq!(midi_to_abc_note(72), "c'");
    }

    #[test]
    fn freq_to_midi_a4() {
        assert_eq!(freq_to_midi(440.0), 69);
    }

    #[test]
    fn freq_to_midi_middle_c() {
        let c4_freq = 261.625_565_3;
        assert_eq!(freq_to_midi(c4_freq), 60);
    }

    #[test]
    fn freq_to_midi_rest() {
        assert_eq!(freq_to_midi(0.0), 0);
    }

    #[test]
    fn load_from_path_dark_moroder() {
        let path = demo_path("dark_moroder");
        let song = EditableSong::load_from_path(&path).expect("dark_moroder should load");

        assert!(!song.title.is_empty(), "title should be populated");
        assert!(song.bpm > 0, "BPM should be positive");
        assert!(!song.tracks.is_empty(), "should have at least one track");
        assert!(!song.instruments.is_empty(), "should have instruments");
        assert!(!song.fx_buses.is_empty(), "should have fx buses");
    }

    #[test]
    fn to_abc_dsl_contains_header_fields() {
        let path = demo_path("dark_moroder");
        let song = EditableSong::load_from_path(&path).expect("dark_moroder should load");
        let dsl = song.to_abc_dsl();

        assert!(dsl.contains("X:1"), "missing X:1 header");
        assert!(dsl.contains(&format!("Q:1/4={}", song.bpm)), "missing tempo");
        assert!(dsl.contains(&format!("%%swing {}", song.swing_pct)), "missing swing");
        assert!(dsl.contains("%%instrument"), "missing instrument directives");
        assert!(dsl.contains("%%effect"), "missing effect directives");
    }

    #[test]
    fn roundtrip_preserves_bpm_and_swing() {
        let path = demo_path("dark_moroder");
        let original = EditableSong::load_from_path(&path).expect("original should load");
        let dsl = original.to_abc_dsl();

        // Write to a temp file so the C parser can re-read it.
        let tmp = std::env::temp_dir().join(format!(
            "memdeck-editor-roundtrip-{}.abc",
            std::process::id()
        ));
        std::fs::write(&tmp, &dsl).expect("should write temp ABC file");

        let reloaded = EditableSong::load_from_path(&tmp);
        let _ = std::fs::remove_file(&tmp);

        let reloaded = reloaded.expect("roundtripped ABC should parse cleanly");
        assert_eq!(reloaded.bpm, original.bpm, "BPM must survive roundtrip");
        assert_eq!(reloaded.swing_pct, original.swing_pct, "swing must survive roundtrip");
    }

    #[test]
    fn roundtrip_preserves_instrument_count() {
        let path = demo_path("dark_moroder");
        let original = EditableSong::load_from_path(&path).expect("original should load");
        let dsl = original.to_abc_dsl();

        let tmp = std::env::temp_dir().join(format!(
            "memdeck-editor-instr-{}.abc",
            std::process::id()
        ));
        std::fs::write(&tmp, &dsl).expect("should write temp ABC file");

        let reloaded = EditableSong::load_from_path(&tmp);
        let _ = std::fs::remove_file(&tmp);

        let reloaded = reloaded.expect("roundtripped ABC should parse cleanly");
        assert_eq!(
            reloaded.instruments.len(),
            original.instruments.len(),
            "instrument count must survive roundtrip"
        );
    }

    #[test]
    fn roundtrip_preserves_arrangement() {
        let path = demo_path("dark_moroder");
        let original = EditableSong::load_from_path(&path).expect("original should load");
        let dsl = original.to_abc_dsl();

        let tmp = std::env::temp_dir().join(format!(
            "memdeck-editor-arr-{}.abc",
            std::process::id()
        ));
        std::fs::write(&tmp, &dsl).expect("should write temp ABC file");

        let reloaded = EditableSong::load_from_path(&tmp);
        let _ = std::fs::remove_file(&tmp);

        let reloaded = reloaded.expect("roundtripped ABC should parse cleanly");
        assert_eq!(
            reloaded.arrangement,
            original.arrangement,
            "arrangement must survive roundtrip"
        );
    }

    #[test]
    fn to_abc_dsl_is_non_empty_for_minimal_song() {
        let song = EditableSong {
            title: "Test Song".to_string(),
            bpm: 120,
            swing_pct: 50,
            patterns: vec![EditablePattern {
                name: "A".to_string(),
                length: 16,
            }],
            arrangement: vec!["A".to_string()],
            instruments: vec![EditableInstrument {
                name: "synth".to_string(),
                waveform: 0,
                amplitude: 64,
                ..Default::default()
            }],
            fx_buses: vec![EditableFxBus::default_for_index(0)],
            tracks: vec![EditableTrack {
                name: "lead".to_string(),
                instrument_ref: "synth".to_string(),
                steps: (0..16)
                    .map(|i| if i % 4 == 0 { EditableStep::note(60) } else { EditableStep::rest() })
                    .collect(),
            }],
        };

        let dsl = song.to_abc_dsl();
        assert!(dsl.contains("X:1"));
        assert!(dsl.contains("Q:1/4=120"));
        assert!(dsl.contains("%%swing 50"));
        assert!(dsl.contains("%%instrument synth"));
        assert!(dsl.contains("%%pattern A length=16"));
        assert!(dsl.contains("%%arrangement A"));
    }
}
