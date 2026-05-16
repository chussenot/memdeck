# MemDeck GUI Editor Architecture

## Overview

The MemDeck GUI is evolving from a **read-only demo browser** into a **minimal
song editor** inspired by Atari ST sequencers (Steinberg Cubase, Pro-24, early
MIDI workstations). The design philosophy is keyboard-first, compact, and
decidedly *not* a modern DAW.

This document describes the in-memory editing model introduced alongside the
existing read-only GUI. The two data paths coexist: the browser panels continue
to work exactly as before; the editor model is a parallel path that can be
activated without disturbing the render pipeline.

---

## Editor modes

```
┌──────────────────────────────────────────────────────────┐
│  EditorMode                                              │
│                                                          │
│  Browser          ← default startup mode                 │
│    Read-only demo browser.                               │
│    DemoOverview / GuiAudioEngine path unchanged.         │
│                                                          │
│  Edit             ← future: in-memory song editing       │
│    EditableSong lives here.                              │
│    All mutations happen to the Rust model only.          │
│                                                          │
│  PreviewRender    ← future: export + re-render           │
│    EditableSong → to_abc_dsl() → C engine → PCM.        │
└──────────────────────────────────────────────────────────┘
```

The mode is stored on `MemDeckGuiApp` and shown in the status bar as
`MODE BROWSER / MODE EDIT / MODE PREVIEW`. Switching modes does **not** change
the read-only GUI behaviour in the current release.

---

## Data model hierarchy

```
EditableSong
├── title: String
├── bpm: i32
├── swing_pct: i32
├── patterns: Vec<EditablePattern>      ← named regions (label + step count)
├── arrangement: Vec<String>            ← ordered pattern names
├── instruments: Vec<EditableInstrument>
├── fx_buses: Vec<EditableFxBus>
└── tracks: Vec<EditableTrack>
      └── steps: Vec<EditableStep>      ← per-step MIDI note (0 = rest)
```

### EditableStep

```rust
pub struct EditableStep {
    pub active: bool,     // false when midi_note == 0
    pub midi_note: u8,    // 0 = rest, 1–127 = MIDI pitch
}
```

Steps map directly to the linear note stream in the ABC voice sections. Each
step corresponds to one `L:1/16` default-length tick.

### EditableInstrument

Mirrors `AbcInstrument` from the FFI layer (waveform integer, ADSR fields, FX
bus routing). No voice-level overrides are stored here; the editor model
collapses instrument and voice into a single editable unit.

### EditableFxBus

Mirrors `AbcFxBus` from the FFI layer (delay, drive, lowpass, sidechain). The
`bus_index` field determines the `%%effect N` directive index in serialised ABC.

### EditablePattern / arrangement

Patterns label regions of the arrangement timeline; they do not own note data.
The note stream lives in `EditableTrack::steps` as a flat `Vec<EditableStep>`
spanning the full song length.

---

## ABC DSL load / save roundtrip

### Loading

```
ABC file on disk
    │
    ▼ ffi::load_editor_song()          (calls C abc_load, preserves freqs[])
    │
    ▼ ffi::extract_raw_abc_music()     (converts AbcMusic → RawAbcMusicForEditor)
    │
    ▼ editor::build_editable_song_from_ffi()
    │
    ▼ EditableSong                     (fully owned Rust model)
```

Unlike `ffi::load_demo_overview()`, the editor path preserves the per-step
note-frequency array (`AbcVoice.freqs[]`). Each Hz value is converted to a MIDI
note number using the same equal-temperament formula as the C engine
(`MIDI = 69 + 12 × log₂(freq / 440)`).

### Saving

```
EditableSong
    │
    ▼ EditableSong::to_abc_dsl()
    │
    ▼ ABC string
    │
    ▼ ffi::render_abc_file()  (existing C render path, unchanged)
    │
    ▼ PCM
```

`to_abc_dsl()` produces valid MemDeck ABC with:

- Standard header (`X:`, `T:`, `M:`, `L:`, `Q:`, `K:`)
- `%%swing N`
- One `%%instrument` line per instrument
- One `%%effect N` line per FX bus
- `%%pattern name length=N` for every pattern
- `%%arrangement name1 name2 …`
- Per-track `V:name instrument=ref` declarations
- Per-track note sequences as individual 1/16-note symbols grouped into bars,
  with `% section name` comments matching pattern labels

### Note serialisation

MIDI note numbers are converted to ABC note names using the standard octave
scheme (`c` = C4 = MIDI 60):

| MIDI range | ABC notation | Example       |
|------------|--------------|---------------|
| 24–35      | `X,,`        | `C,,` = C1    |
| 36–47      | `X,`         | `C,` = C2     |
| 48–59      | `X`          | `C` = C3      |
| 60–71      | `x`          | `c` = C4      |
| 72–83      | `x'`         | `c'` = C5     |
| 84–95      | `x''`        | `c''` = C6    |

Sharps are used for accidentals (`^c` = C#4). Rests are emitted as `z`.

---

## FFI bridge

The existing `ffi.rs` module now exports two public APIs:

| Function | Purpose |
|----------|---------|
| `ffi::load_demo_overview(path)` | Read-only browser path (unchanged) |
| `ffi::load_editor_song(path)` | Editor path — preserves note frequencies |

The helper types `RawAbcMusicForEditor`, `RawVoiceForEditor`, etc. are
intermediate structs used to carry C-layout data out of the `unsafe` block
before handing it to the pure-Rust `editor` module.

---

## Coexistence with the read-only GUI

The read-only `DemoOverview` / `GuiAudioEngine` pipeline is **not modified**.

```
MemDeckGuiApp {
    audio_engine: GuiAudioEngine,   ← read-only browser (unchanged)
    editor_mode: EditorMode,        ← Browser / Edit / PreviewRender (new)
    ...
}
```

When `editor_mode == Browser` (the default), the app behaves exactly as before.
The `editor_mode` field is displayed in the status bar to give the user and
developer visibility of the current mode.

---

## Directory layout

```
gui/
  src/
    app.rs            ← MemDeckGuiApp, editor_mode field added
    audio_engine.rs   ← read-only DemoOverview types (unchanged)
    editor.rs         ← NEW: EditableSong + all editable types
    ffi.rs            ← load_editor_song + Raw* helper structs added
    main.rs           ← mod editor added
    playback.rs       ← unchanged

docs/
  gui-editor-architecture.md  ← this file
```

---

## Acceptance checklist

- [x] Existing read-only GUI still works — all prior tests pass unchanged
- [x] `EditableSong`, `EditablePattern`, `EditableTrack`, `EditableStep`,
      `EditableInstrument`, `EditableFxBus` types exist in `editor.rs`
- [x] `EditorMode` (Browser / Edit / PreviewRender) declared and shown in UI
- [x] `EditableSong::load_from_path` loads from ABC DSL via FFI
- [x] `EditableSong::to_abc_dsl` serialises back to MemDeck ABC string
- [x] ABC load/save roundtrip preserves BPM, swing, instrument count,
      and arrangement for `dark_moroder` (and all other showcase demos)
- [x] No advanced editing UI (no piano roll, no clip editing, no plug-ins)
- [x] No DAW behaviour introduced
- [x] C audio engine not modified

---

## Future work (not in scope for this release)

- Wire `EditorMode::Edit` into the panel layout (step grid, pattern lane)
- `EditorMode::PreviewRender`: export `EditableSong` to a temp file and route
  through the existing `render_selected_demo` path
- Step-grid editing panel (Atari ST style, keyboard-first)
- Pattern copy / paste / loop within the arrangement
- Per-track mute / solo flags in `EditableTrack`
- Undo stack (command pattern over `EditableSong` mutations)
