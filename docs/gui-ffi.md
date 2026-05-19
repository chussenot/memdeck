# MemDeck GUI FFI and Playback Boundary

## Rust/C boundary

Only `gui/src/ffi.rs` crosses the unsafe boundary.

Rust calls:

- `audio_engine_render_abc_file(path, sample_rate, out_len, out_stats)`
- `audio_engine_free_buffer(buffer)`
- `abc_load(path, music_out)`

No UI code performs direct FFI calls.

## Metadata extraction

`abc_load` is used to build the read-only GUI overview before any render occurs. The FFI layer converts `AbcMusic` into:

- `DemoOverview`
- `TrackOverview`
- `FxBusOverview`
- `PatternBlock`
- `StepState`

This is where waveform names, inferred beat markers, arrangement blocks, and bus details are normalized for the GUI.

## Memory ownership

Rendered PCM is allocated by the C engine.

1. Rust validates the path and converts it to a C string.
2. Rust calls the C renderer.
3. The returned raw pointer is wrapped in an owned helper with `Drop`.
4. PCM is copied into Rust-owned memory.
5. The original C allocation is always freed.
6. The latest `AudioRenderStats` snapshot is cached for the UI.

## Playback boundary

`gui/src/playback.rs` is intentionally outside the FFI layer.

- rendered PCM is copied into Rust-owned memory and streamed directly with `cpal`
- playback progress is tracked from consumed sample cursor over full render length
- stop/poll always clean up stream ownership and playback runtime state

## Limitations

- playback depends on host output device availability and a valid default stream config
- no separate low-latency GUI mixer is introduced; playback is output-stream only
- metadata is for inspection only; the GUI never edits the C structures
