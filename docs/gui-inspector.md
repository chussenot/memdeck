# MemDeck GUI Inspector

The inspector surface is split into two dedicated read-only panels driven by the selected track.

## Track selection

- `Up` / `Down` change the selected track whenever focus is outside the Demo Browser
- the Pattern Overview highlights the selected row
- both inspectors stay locked to the same selected track

## Instrument Inspector

The instrument panel shows selected-track voice metadata from `TrackOverview`:

- waveform glyph + waveform name
- amplitude meter
- duty meter
- gate meter
- ADSR scope
- glide meter
- vibrato depth/rate meter
- detune meter
- preset label when present

All visuals are non-interactive. They are diagnostic only.

## FX Inspector

The FX panel shows the selected track's routed bus from `FxBusOverview`:

- bus index
- active / bypass state
- delay meter with steps / feedback / mix
- drive meter
- low-pass meter
- sidechain meter with release
- bus mix meter

## Data source

`gui/src/ffi.rs` extracts track and bus detail from `AbcMusic` and normalizes it into GUI-facing structs. The GUI does not mutate any of this data.

## Explicit non-goals

- no knobs, sliders, or editable fields
- no oscilloscope editing surface
- no tracker instrument editor
- no DAW mixer behavior
