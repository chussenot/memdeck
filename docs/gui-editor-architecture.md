# MemDeck GUI Editor Architecture

## Overview

The GUI now runs three explicit modes:

- **Browser mode**: read-only demo browser + render inspector (default startup behavior).
- **Edit mode**: editable workflow for arrangement + pattern step editing.
- **Preview mode**: editable song rendered through the existing C engine for transport preview.

Read-only browser behavior is preserved and can be re-entered at any time.

## Editor modules

```
gui/src/editor/
  mod.rs
  model.rs
  state.rs
  abc_load.rs
  abc_save.rs
  validation.rs
```

### `model.rs`
Defines:

- `EditableSong`
- `EditablePattern`
- `EditableTrack`
- `EditableStep`
- `EditableInstrument`
- `EditableFxBus`
- `EditableArrangement`
- `EditableArrangementBlock`

`EditableSong` carries `title`, `tempo`, `swing`, `patterns`, `tracks`, `instruments`, `fx_buses`, `arrangement`, `dirty`, and `source_path`.

`EditableStep` carries:

- `active`
- `midi_note`
- `velocity`
- `gate_percent`
- `accent`
- `fx_trigger`

### `state.rs`
Defines `EditorState`:

- `mode: Browser | Edit | Preview`
- `selected_pattern`
- `selected_track`
- `selected_step`
- `selected_arrangement_block`
- `dirty`
- `last_saved_path`
- `last_error`

### `abc_load.rs`
- Loads editable songs from ABC through existing FFI/C parser path.
- Converts raw frequencies to MIDI steps.
- Builds arrangement blocks from `%%arrangement` + `%%pattern` metadata.
- Preserves single-percent comments where practical.
- Reads optional per-step metadata directives (`%%mdstep`) for velocity/gate/accent/fx flags.

### `abc_save.rs`
- Deterministic ABC serialization from `EditableSong`.
- Emits standard ABC header + MemDeck directives.
- Emits `%%pattern`, `%%arrangement`, voice declarations, and per-track note bars.
- Emits optional `%%mdstep` directives for non-default per-step edit metadata.
- Supports save-to-path and clean dirty/source-path updates.

### `validation.rs`
- Validates tempo/swing ranges.
- Validates non-empty title/tracks/patterns/arrangement.
- Validates arrangement references existing patterns.

## GUI integration

`MemDeckGuiApp` now includes:

- `editor_state: EditorState`
- `editable_song: Option<EditableSong>`
- explicit focusable `Pattern Editor` panel
- editable Instrument/FX inspectors in Edit/Preview mode
- step clipboard for copy/cut/paste operations

New top-bar actions:

- **New Song**
- **Duplicate Demo as Editable**
- **Open Editable Song**
- **Browser Mode**

Focus areas now include:

- Demo Browser
- Render Stats
- Waveform View
- Pattern Overview (arrangement in edit/preview modes)
- Pattern Editor
- Instrument Inspector
- FX Inspector

## Preview render path

Editable preview render stays on the existing C engine path:

`EditableSong -> ABC DSL -> ffi::render_abc_file -> PCM`

No second Rust audio engine is introduced.

## Screenshot references

- Browser mode: `docs/screenshots/gui-browser-mode.png`
- Edit mode + arrangement: `docs/screenshots/gui-edit-mode-arrangement.png`
- Pattern editor selected cell: `docs/screenshots/gui-pattern-editor-selected-cell.png`
- Pattern editor after edits: `docs/screenshots/gui-pattern-editor-edited-notes.png`
- Instrument inspector focused: `docs/screenshots/gui-edit-mode-instrument.png`
- FX inspector focused: `docs/screenshots/gui-edit-mode-fx.png`
- Preview mode: `docs/screenshots/gui-preview-mode.png`
- Deterministic capture process: `docs/gui-screenshot-guide.md`

## Tests

Editor tests cover:

- editable step defaults
- step toggle + note/octave behavior
- load simple ABC
- save simple ABC
- load/save roundtrip
- accent/fx step metadata roundtrip
- invalid ABC returns clean error
