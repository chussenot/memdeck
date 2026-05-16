# MemDeck GUI Arrangement Editor

## Goal

Provide a minimal Cubase/Atari-inspired arrangement editor without turning MemDeck into a modern DAW.

## Scope

Included:

- track list on the left
- linear block arrangement on the right
- block selection, creation, duplication, deletion
- block reordering (keyboard)
- pattern rename
- pattern length edits
- tempo/swing edits
- preview render through existing C engine
- open selected arrangement block directly into Pattern Editor

Explicitly out of scope:

- piano roll editing
- audio clip editing
- plugin architecture
- replacement of egui/eframe
- replacement of C audio engine

## Workflow

1. Enter editable workflow using **New Song**, **Duplicate Demo as Editable**, or **Open Editable Song**.
2. Focus arrangement panel (`A`).
3. Edit blocks via keyboard-first commands.
4. Open selected block (`Enter`) to switch focus to Pattern Editor.
5. Save to ABC (`Ctrl+S` / `Ctrl+Shift+S`).
6. Render preview (`Ctrl+R`) and play via existing transport.

## Data flow

`EditableSong -> serialize_editable_song() -> temporary/target .abc -> ffi::render_abc_file() -> PCM`

This keeps one audio engine path and preserves deterministic browser behavior.

## Screenshot

- `docs/screenshots/gui-edit-mode-arrangement.png`
