# MemDeck GUI Layout

The GUI remains a compact Atari/ST-style workstation with keyboard-first interaction.

## Runtime modes

- **Browser**: read-only demo browsing and inspection (unchanged behavior).
- **Edit**: editable arrangement workflow.
- **Preview**: editable song rendered through existing C engine for preview playback.

## Primary screen

```text
+----------------------------------------------------------------------------+
| MEMDECK SOUND MACHINE                                          FOCUS • ... |
| [NEW SONG] [DUPLICATE DEMO AS EDITABLE] [OPEN EDITABLE SONG] [PATH INPUT] |
+---------------------------------------+------------------------------------+
| DEMO BROWSER                          | RENDER STATS                       |
+----------------------------------------------------------------------------+
| WAVEFORM                                                                   |
+----------------------------------------------------------------------------+
| PATTERN OVERVIEW (Browser mode) / ARRANGEMENT EDITOR (Edit/Preview modes) |
+-----------------------------------+----------------------------------------+
| INSTRUMENT INSPECTOR              | FX INSPECTOR                           |
+----------------------------------------------------------------------------+
| STATUS LINE                                                                  |
+----------------------------------------------------------------------------+
```

## Arrangement editor panel (Edit/Preview)

- left: track list
- right: horizontal arrangement blocks
- selected block highlight
- cursor + selected track readout
- tempo/swing controls
- block length controls (`-LEN`, `+LEN`)
- optional rename row when rename is active

## Design rules

- dark limited palette
- strong rectangular borders and grid lines
- monospace typography
- visible focus indicators
- no glossy widgets, no modern DAW styling
