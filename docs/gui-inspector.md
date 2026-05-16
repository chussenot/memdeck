# MemDeck GUI Inspector

The inspector surface is split into two panels driven by the selected track.

## Track selection

- `Up` / `Down` change the selected track whenever focus is outside the Demo Browser
- the Pattern Overview highlights the selected row
- both inspectors stay locked to the same selected track

## Instrument Inspector

### Browser mode

Read-only metadata from `TrackOverview`:

- waveform glyph + waveform name
- amplitude/duty/gate meters
- ADSR scope
- glide/vibrato/detune meters
- preset label when present

### Edit/Preview modes

Editable controls for the selected track's instrument:

- track instrument assignment
- waveform selection (square/pulse/triangle/noise)
- amplitude/duty/gate
- ADSR sliders
- glide and vibrato amount

Edits update `EditableSong`, mark dirty state, and invalidate stale preview render state.

## FX Inspector

### Browser mode

Read-only routed bus metadata from `FxBusOverview`:

- bus index
- active / bypass state
- delay/drive/low-pass/sidechain/bus-mix meters

### Edit/Preview modes

Editable routing and bus controls:

- selected track bus routing (`ROUTED BUS`)
- delay steps/feedback/mix
- drive, low-pass
- sidechain amount/release
- bus mix
- missing-bus recovery (`CREATE MISSING BUS`)

## Data source

- Browser mode: `gui/src/ffi.rs` overview structs (`TrackOverview`, `FxBusOverview`)
- Edit/Preview: `EditableSong` instrument/fx structures in `gui/src/editor/model.rs`

## Explicit non-goals

- no piano roll
- no DAW mixer/timeline behavior
- no plugin architecture
