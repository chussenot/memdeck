# MemDeck GUI Runtime

MemDeck's Rust GUI is a thin runtime shell over the existing C renderer. It stays read-only and keeps the C engine as the source of truth for demo parsing, sequencing, mixing, FX, and deterministic PCM generation.

## Runtime architecture

```mermaid
flowchart LR
    GUI[egui / eframe runtime screen] --> WRAPPER[gui/src/audio_engine.rs]
    WRAPPER --> FFI[gui/src/ffi.rs]
    FFI --> ENGINE[src/audio_engine.c]
    ENGINE --> ABC[ABC loader / parser]
    ABC --> SONG[SeqSong]
    SONG --> MIXER[mixer]
    MIXER --> FX[FX buses]
    FX --> PCM[deterministic PCM buffer]
```

## Runtime flow

```mermaid
sequenceDiagram
    participant User
    participant GUI as Rust GUI
    participant FFI as Rust FFI wrapper
    participant Engine as C audio_engine
    participant Seq as SeqSong
    participant Mixer as mixer + FX
    participant PCM as PCM buffer

    User->>GUI: select demo
    User->>GUI: Enter render
    GUI->>FFI: load metadata + render_abc_file(path)
    FFI->>Engine: abc_load(path)
    FFI->>Engine: audio_engine_render_abc_file(path)
    Engine->>Seq: build deterministic sequence
    Seq->>Mixer: render arranged steps
    Mixer->>PCM: mix + FX + clip stats
    PCM-->>FFI: PCM + AudioRenderStats
    FFI-->>GUI: safe Rust state
    GUI-->>User: stats + waveform + pattern overview
```

## Stable runtime responsibilities

- `gui/src/ffi.rs` contains the unsafe boundary.
- `gui/src/audio_engine.rs` exposes safe demo metadata and render helpers.
- `gui/src/playback.rs` handles simple OS-level playback of rendered WAV output.
- `gui/src/app.rs` owns the one-screen keyboard-first runtime UI.

## Runtime feedback

The runtime screen shows:

- render duration
- sample count
- clipping count
- peak level
- checksum
- render state
- invalid ABC/load failures

## Screenshots

- Main runtime screen: `docs/screenshots/gui-runtime-main.png`
- Waveform / pattern overview: `docs/screenshots/gui-runtime-overview.png`
