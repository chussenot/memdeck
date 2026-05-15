# MemDeck GUI Layout

The GUI is now a single-screen, read-only composition workstation with a fixed panel hierarchy and Atari/ST-style rhythm.

## Primary screen

```text
+----------------------------------------------------------------------------+
| MEMDECK SOUND MACHINE                                         FOCUS • ...  |
+---------------------------------------+------------------------------------+
| DEMO BROWSER                          | RENDER STATS                       |
| 01 DARK MORODER                       | tempo / swing / steps              |
| 02 NEON NIGHTDRIVE                    | render + playback state            |
| ...                                   | checksum / duration                |
+---------------------------------------+------------------------------------+
| WAVEFORM                                                                   |
| PCM minimap | clipping markers | playback cursor                           |
+----------------------------------------------------------------------------+
| PATTERN OVERVIEW                                                           |
| track labels | arrangement blocks | beat grid | active steps | playhead     |
+-----------------------------------+----------------------------------------+
| INSTRUMENT INSPECTOR              | FX INSPECTOR                           |
| waveform / ADSR / duty / vibrato  | delay / drive / low-pass / sidechain   |
+----------------------------------------------------------------------------+
| STATUS LINE                                                                  |
+----------------------------------------------------------------------------+
```

## Panel roles

### Demo Browser
- fixed showcase demo catalog
- keyboard-first selection
- load/render availability feedback
- no editing controls

### Render Stats
- demo identity and tempo metadata
- deterministic render metrics
- explicit render/playback/focus state chips
- selected track summary

### Waveform View
- rendered PCM minimap
- clip marker overlay
- playback cursor
- lightweight draw path (chunked envelope, no heavy effects)
- full-width strip beneath demo/stats for quick transport context

### Pattern Overview
- arrangement blocks in the header strip
- beat grid + pattern boundaries
- active step cells with accent / FX trigger markers
- selected-track highlight for inspector coherence

### Instrument Inspector
- selected track only
- waveform glyph
- compact ADSR scope
- read-only meters for amp, duty, gate, glide, vibrato, detune

### FX Inspector
- selected track's routed FX bus only
- read-only meters for delay, drive, low-pass, sidechain, bus mix
- clear bus index and active/bypass state

### Status Line
- current runtime message with severity color
- selected demo + selected track + focused panel snapshot
- render + playback state snapshot
- last error (when present)
- shortcut reminder strip
- no hidden transport state

## Focus model

`Tab` cycles through all six panels in this order:

1. Demo Browser
2. Render Stats
3. Waveform View
4. Pattern Overview
5. Instrument Inspector
6. FX Inspector

Direct focus keys:

- `D` demo browser
- `S` render stats
- `W` waveform view
- `P` pattern overview
- `I` instrument inspector
- `F` FX inspector

`Up` / `Down` change the demo when the browser is focused, and change the selected track in every other panel.

## Design rules

- dark, limited palette
- crisp rectangular borders only
- monospace typography only
- no gradients, glossy widgets, or animation systems
- panel separation always visible
- read-only workstation behavior only
