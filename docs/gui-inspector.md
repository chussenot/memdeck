# MemDeck GUI Inspector

The MemDeck GUI inspector surfaces runtime metadata about the selected demo. It is strictly read-only — no parameter editing is possible.

## Current Inspector Surface (Render Stats Panel)

The `RENDER STATS` panel acts as the primary inspector. It displays:

| Field | Source | Description |
|---|---|---|
| demo | `DemoEntry.key` | Short name of the selected demo |
| bpm | `DemoOverview.bpm` | Tempo in beats per minute |
| swing | `DemoOverview.swing_pct` | Swing percentage (50 = straight) |
| duration | `AudioRenderStats.duration_ms` | Total rendered duration in ms |
| samples | `AudioRenderStats.sample_count` | Total PCM sample count |
| clipping | `AudioRenderStats.clipping_count` | Number of clipped samples (red if > 0) |
| peak | `AudioRenderStats.peak` | Peak PCM amplitude value |
| min/max | `AudioRenderStats.min_sample` / `max_sample` | Amplitude range |
| render ms | `AudioRenderStats.render_time_ms` | C engine wall time for PCM generation |
| checksum | `AudioRenderStats.checksum` | Deterministic render fingerprint |
| render | derived | `success` (green) or `pending` (dim) |

Stats are populated after a successful render (`Enter`). Fields show `--` before the first render.

A compact summary line below the grid shows:

```
PATTERNS N • STEPS N • TRACKS N
```

And if voices exceed the renderer limit:

```
RENDERER SHOWS FIRST N TRACKS • N HIDDEN
```

## Playback State

The panel also shows the current playback state:

- `STATE  PLAYING` (ACCENT green) — audio process is active
- `STATE  IDLE` (TEXT_DIM) — no audio process running

## No Instrument / FX Inspector

The current implementation does not render a dedicated instrument or FX inspector panel. The ABC metadata for instruments (`AbcInstrument`) and FX buses (`AbcFxBus`) is parsed via `ffi.rs` but not currently forwarded to the UI beyond the `instrument` field on each `TrackOverview` (shown in the pattern label column as `track / instrument_ref`).

Future expansion of the inspector (without editing) would surface:

- waveform type (square/pulse/triangle/noise)
- ADSR envelope values
- gate/duty cycle
- vibrato cents and rate
- glide ms
- FX bus assignment
- FX bus parameters (delay, drive, lowpass, sidechain)

This would require extending `DemoOverview` with per-voice and per-bus detail structs, populated from `AbcInstrument` and `AbcFxBus` in `build_demo_overview()`.

## Design Constraints

- All fields are labels — no interactive widgets.
- No editing of any parameter.
- No ADSR graph or oscilloscope widget.
- Compact monospace grid only.
