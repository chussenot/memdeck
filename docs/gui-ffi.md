# MemDeck GUI FFI and Playback Boundary

## Rust/C boundary

The GUI crate (`gui/`) calls the C audio engine through `gui/src/ffi.rs` only.

Rust invokes:

- `audio_engine_render_abc_file(path, sample_rate, out_len, out_stats) -> *mut u8`
- `audio_engine_free_buffer(buffer)`
- `abc_load(path, music_out)`

No other GUI module performs direct FFI calls.

## Memory ownership

Rendered audio memory is allocated by the C engine and returned as a raw pointer.

- C owns allocation.
- Rust must always release returned buffers with `audio_engine_free_buffer`.
- `ffi.rs` wraps returned pointers in an internal owned buffer type with `Drop`, so release still happens on early errors.

## Buffer lifecycle

Render path (`ffi::render_abc_file`):

1. Validate path (exists, file, UTF-8, no interior NUL for C string).
2. Call C renderer.
3. Wrap raw pointer + length in owned Rust wrapper.
4. Copy to `Vec<u8>` for GUI/runtime ownership.
5. Free original C buffer.
6. Store latest `AudioRenderStats` snapshot.

Failure behavior:

- Null pointer, invalid length, invalid path, CString conversion failure, and parser/render errors return `Result::Err`.
- Failed render paths do not leak the C buffer.

## Render flow

1. `GuiAudioEngine::demo_catalog` discovers showcase demo files and tries `ffi::load_demo_overview`.
2. `GuiAudioEngine::render_demo` calls `ffi::render_abc_file`.
3. App stores render output (`RenderState`) separately from selected metadata.
4. Status line shows success/error and checksum/stats when available.

## Playback flow

Playback is intentionally simple (`gui/src/playback.rs`):

1. GUI requests playback of rendered PCM (`start_pcm`).
2. PCM is written as a temporary mono 8-bit WAV at engine sample rate.
3. Platform command is spawned (`afplay`, PowerShell `SoundPlayer`, or `aplay`).
4. UI polls process status (`poll`) without blocking the frame loop.
5. Stop uses process kill + cleanup (`stop`), and temp file is removed.

`PlaybackState` exposed to UI:

- `Stopped`
- `Playing`
- `Error(String)`

## Known limitations

- Playback relies on external system audio commands.
- WAV wrapping is used for portability; no real-time low-latency mixer is implemented in Rust.
- GUI keeps C engine as source of truth for rendering and statistics.
- This layer is render/playback only (no tracker, editor, piano roll, or DAW behavior).
