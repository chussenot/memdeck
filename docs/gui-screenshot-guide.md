# MemDeck GUI Screenshot Guide

This guide defines deterministic screenshot captures for the editable workflow.

## Global capture rules

- Window size: default GUI viewport (`1200x760`)
- Theme/layout: default Atari/ST dark theme
- Demo seed: `dark_moroder`
- Playback: stopped before capture (default in screenshot boot)
- Capture command shape:

```sh
EFRAME_SCREENSHOT_TO=<target.png> xvfb-run -a cargo run --manifest-path gui/Cargo.toml -- --screenshot
```

## Required scenarios

### 1) Browser Mode
- File: `docs/screenshots/gui-browser-mode.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_MODE=browser`
  - `MEMDECK_GUI_BOOT_FOCUS=demos`
  - `MEMDECK_GUI_BOOT_RENDER=1`

### 2) Arrangement Edit Mode
- File: `docs/screenshots/gui-edit-mode-arrangement.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_FOCUS=arrangement`

### 3) Pattern Editor Focused
- File: `docs/screenshots/gui-pattern-editor-selected-cell.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_FOCUS=pattern-editor`

### 4) Instrument Inspector Focused
- File: `docs/screenshots/gui-edit-mode-instrument.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_FOCUS=instrument`

### 5) FX Inspector Focused
- File: `docs/screenshots/gui-edit-mode-fx.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_FOCUS=fx`

### 6) Preview Mode
- File: `docs/screenshots/gui-preview-mode.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=preview`
  - `MEMDECK_GUI_BOOT_FOCUS=waveform`

### 7) Dirty-state visible (Edit mode)
- File: `docs/screenshots/gui-dirty-state.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_PATTERN_EDITS=1`

### 8) Save dialog
- File: `docs/screenshots/gui-save-dialog.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_DIALOG=save-as`

### 9) Open dialog + recent songs
- File: `docs/screenshots/gui-open-dialog-recent.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_DIALOG=open`

### 10) Duplicated demo workflow
- File: `docs/screenshots/gui-duplicated-demo-workflow.png`
- Env:
  - `MEMDECK_GUI_BOOT_DEMO=dark_moroder`
  - `MEMDECK_GUI_BOOT_EDITABLE=duplicate`
  - `MEMDECK_GUI_BOOT_MODE=edit`
  - `MEMDECK_GUI_BOOT_FOCUS=demos`

## Optional edited-pattern snapshot

- File: `docs/screenshots/gui-pattern-editor-edited-notes.png`
- Add:
  - `MEMDECK_GUI_BOOT_PATTERN_EDITS=1`
