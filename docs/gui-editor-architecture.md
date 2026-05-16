# MemDeck GUI Editor Architecture

## Overview

The GUI now has two concurrent paths:

- **Browser mode**: existing read-only demo browser/render inspector.
- **Edit/Preview modes**: editable song model and arrangement editor.

The read-only path is preserved and remains the default startup mode.

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

### `abc_save.rs`
- Deterministic ABC serialization from `EditableSong`.
- Emits standard ABC header + MemDeck directives.
- Emits `%%pattern`, `%%arrangement`, voice declarations, and per-track note bars.
- Supports save-to-path and clean dirty/source-path updates.

### `validation.rs`
- Validates tempo/swing ranges.
- Validates non-empty title/tracks/patterns/arrangement.
- Validates arrangement references existing patterns.

## GUI integration

`MemDeckGuiApp` now includes:

- `editor_state: EditorState`
- `editable_song: Option<EditableSong>`

New top-bar actions:

- **New Song**
- **Duplicate Demo as Editable**
- **Open Editable Song**
- **Browser Mode**

## Preview render path

Editable preview render stays on the existing C engine path:

`EditableSong -> ABC DSL -> ffi::render_abc_file -> PCM`

No second Rust audio engine is introduced.

## Tests

Editor tests cover:

- load simple ABC
- save simple ABC
- load/save roundtrip
- invalid ABC returns clean error
