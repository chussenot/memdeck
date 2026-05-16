# MemDeck GUI Pattern Editor

## Goal

Provide a compact Atari/ST-inspired step grid for editing the currently selected arrangement block without adding piano-roll or DAW timeline behavior.

## Scope

Included:

- track-by-step grid editor
- selected-cell cursor
- note/rest visibility per step
- accent and FX trigger markers
- per-step velocity and gate editing
- keyboard-first step editing with mouse select/toggle support
- selection clamping across arrangement block changes

Out of scope:

- piano roll
- drag/drop editing
- advanced timeline editing
- DAW transport redesign

## Focus and integration

- Focus area: **Pattern Editor**
- Focus shortcuts: `E` or `G`
- Arrangement integration: edits apply to the selected arrangement block/pattern only

## Keyboard controls

| Key | Action |
| --- | --- |
| `Arrow keys` | Move cursor (track/step) |
| `Space` or `Enter` | Toggle step on/off |
| `+` / `-` | Octave up/down |
| `A` | Toggle accent |
| `F` | Toggle FX trigger |
| `G` | Cycle gate value |
| `V` | Cycle velocity value |
| `C` | Copy step |
| `X` | Cut step |
| `P` | Paste step |
| `Esc` | Return focus to Arrangement Editor |

## Mouse controls

- click cell: select cursor
- double-click cell: toggle step

## Save/render workflow

1. Edit steps in Pattern Editor.
2. Song is marked dirty.
3. Save with `Ctrl+S` / `Ctrl+Shift+S` (ABC DSL + `%%mdstep` metadata for non-default step fields).
4. Render preview with `Ctrl+R` through existing C engine path.
5. Preview mode updates waveform/stats/pattern panels from rendered editable PCM.
6. Any edit after preview render invalidates stale preview and returns to Edit mode.

## Current limitations

- step velocity/gate/accent/fx metadata is persisted via `%%mdstep` directives
- C engine playback remains driven by existing ABC parse/build behavior
- no auto-render on every keystroke

## Screenshots

- selected cell: `docs/screenshots/gui-pattern-editor-selected-cell.png`
- edited notes/markers: `docs/screenshots/gui-pattern-editor-edited-notes.png`
