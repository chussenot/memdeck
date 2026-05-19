# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

Two products in one tree:

1. **MemDeck TUI** (`bin/memdeck-tui`) — an ncurses-based memorized-deck trainer for card magicians.
2. **MemDeck audio engine** — a deterministic, integer-fixed-point synth/sequencer in C that powers the TUI's sound effects, a CLI showcase renderer, a Rust GUI for composing/inspecting songs, and an MCP server for agent-driven authoring.

The audio engine is the part most edits touch. **All audio rendering — TUI sounds, CLI tools, `make test`, the Rust GUI, and the MCP server — goes through the same C entry point (`audio_engine_render_abc_file` in `src/audio_engine.c`).** Keeping that path single-source is a load-bearing invariant; see "Render-path consistency" below.

## Build and test

```sh
make all              # TUI (bin/memdeck-tui)
make test             # full C suite: audio DSP, ABC parser, sequencer, cards/stacks/scoring
make test-abc         # just the ABC parser + showcase regression (bit-exact PCM checksums)
make test-audio-seq   # just sequencer regression
make gui-check        # cargo check on the Rust GUI crate
make gui-test         # cargo test on the Rust GUI crate (run with --test-threads=1 if you see races)
make gui-run          # launch the Rust GUI
make render-demos     # render every showcase ABC to PCM and print {duration, checksum, clipping}
make play-demo DEMO=name [WAV=1]   # render one song; name resolves through tests/play_demo.c kDemos
make mcp              # build the MCP server (bin/memdeck-mcp)
make mcp-smoke        # pipe a few JSON-RPC requests through it
```

The Rust GUI's `build.rs` compiles `src/*.c` into the binary, so a C header change must rebuild the Rust side too. If the GUI ever shows mismatched struct data (e.g. `TRACKS 0` on a freshly-added song), the symptom is a stale C build inside `gui/target/`: `cargo clean --manifest-path gui/Cargo.toml && make gui-run`.

On Linux the GUI's `cpal` audio backend needs ALSA dev headers: `mise run gui-system-deps` (declared in `.mise.toml`) or `sudo apt install libasound2-dev`.

## High-level architecture

### The audio render pipeline

```
.abc text  --abc_load-->  AbcMusic  --abc_build_seq_song-->  SeqSong
                                                                  |
                                       audio_mix_render_song /  audio_engine_render_song
                                                                  v
                                                         u8 PCM @ 22050 Hz mono
```

- `src/abc.c` parses ABC notation (with MemDeck's `%%` directive extensions for instruments, FX buses, patterns, arrangement) into an `AbcMusic` struct.
- `abc_build_seq_song` bridges that into the runtime `SeqSong` (timeline, tracks, steps, FX bus config).
- `src/audio_mix.c` walks the timeline, instantiates voices via `src/audio_dsp.c`, sums them through the FX chain in `src/audio_fx.c`.
- `src/audio_engine.c` is the public entry point used by every consumer.

`docs/audio-architecture.md`, `docs/audio-abc-dsl.md`, and `docs/audio-fx.md` cover this in depth — read them before touching the engine.

### Two struct layers, kept in sync

The engine has two parallel struct families that look similar but live at different layers:

- `AbcMusic` / `AbcFxBus` / `AbcInstrument` etc. in `src/memdeck.h` — the parser's view (what's in the source file).
- `SeqSong` / `SeqFxBus` / `SeqInstrument` etc. in `src/audio_seq.h` — the runtime view (what the mixer consumes).

`abc_build_seq_song` is the bridge. **When you add a field (e.g. the Moog ladder fields), it must appear in both struct families, in the bridge function, in the Rust FFI mirrors at `gui/src/ffi.rs`, in the editor's mirror at `gui/src/editor/model.rs`, and the editor's load/save round-trip at `gui/src/editor/{abc_load,abc_save}.rs`.** Forgetting any of these surfaces as silent data loss, not a compile error.

### Engine caps (canonical, both C and Rust mirrors must match)

From `src/audio_seq.h` and `src/memdeck.h`:

| Constant | Value | Meaning |
| --- | ---: | --- |
| `SEQ_MAX_STEPS` | 64 | per-pattern step cap |
| `SEQ_MAX_TRACKS` | 8 | per-pattern voice cap |
| `SEQ_MAX_PATTERNS` | 16 | unique pattern instances in the song timeline |
| `SEQ_MAX_ARRANGEMENT` | 16 | arrangement slots (sequencer-side) |
| `SEQ_MAX_TIMELINE_STEPS` | 1024 | total render steps = 64 × 16 |
| `SEQ_MAX_FX_BUSES` | 4 | FX buses |
| `ABC_MAX_VOICES` | 8 | parser-side voice cap |
| `ABC_MAX_PATTERNS` | 16 | parser-side pattern cap |
| `ABC_MAX_ARRANGEMENT` | 32 | parser-side arrangement-name cap |
| `SAMPLE_RATE_ABC` | 22050 | render rate (mono u8) |

At `Q:1/4=100`, `L:1/8`, 1024 timeline steps ≈ 5:07 — the practical ceiling for a single song. At `BPM=140`, the same cap gives 3:39.

The Rust mirrors of these caps live in `gui/src/ffi.rs` as `const SEQ_MAX_TRACKS: usize = 8;` etc. Bumping a C cap requires bumping the matching Rust const **and** the corresponding struct layout (e.g. `patterns: [AbcPatternDef; ABC_MAX_PATTERNS]`).

### ABC arrangement is consumed linearly

A non-obvious parser behavior, easy to misread: `%%arrangement A A B C ...` doesn't reuse voice content. Each arrangement slot advances the per-voice source cursor by its pattern's `length`. Reusing the name "A" twice still reads two distinct chunks from the voice's bars — the pattern name only looks up the slot's `length`. A voice needs total source steps = sum of arrangement-slot lengths, or rendering aborts.

Practical consequence: a 5-minute song at L:1/8 BPM=100 needs ~128 bars of source per voice. See `data/music/glass_anthem.abc` for an example with 16 distinct sections.

### FX bus chain order

`drive → lowpass → ladder → delay → sidechain → mix` (defined in `audio_fx_bus_process`). The Moog ladder sits between the existing one-pole lowpass and the delay so it can shape resonance without losing the cheap tone-tilt option. ADR-0002 in `docs/` explains why the ladder is pure C in-house rather than a Rust crate or vendored C++.

### Showcase regression goldens

`tests/test_abc.c` locks the **bit-exact** FNV-1a-64 PCM checksum and a clipping-count ceiling for ten showcase demos (`dark_moroder` through `hypersleep_dream`). Any change that perturbs the output of those demos requires re-rendering with `make render-demos`, then updating the `expected_checksum` and (if needed) `max_clipping` literals in `tests/test_abc.c:174`. New songs added to `data/music/` are NOT auto-locked — they only appear in the showcase list via `tests/render_demos.c` and `tests/play_demo.c`.

When adding an engine feature (e.g. ladder fields, new directive), prefer making it a **no-op by default**: existing demos should render bit-identical until they opt in via a directive. That keeps showcase tests passing without a golden refresh in the same commit.

### The Rust GUI is a shell over the C engine

`gui/` is an eframe/egui app that:

- mirrors the C structs over FFI (`gui/src/ffi.rs`)
- enumerates `data/music/*.abc` dynamically (`gui/src/audio_engine.rs::scan_showcase_demos`) and skips files whose stem starts with `menu` (UI sounds)
- renders demos by calling the C engine through FFI; never reimplements DSP
- supports an editable mode where edits round-trip to ABC text and back through the same parser (`gui/src/editor/`)
- plays back PCM in-process via `cpal` (ADR-0001) — the playback layer is Rust-side, but DSP stays in C

When the editor writes a preview, it goes to `/tmp/memdeck-edit-preview-{pid}-{thread_id}.abc`. The thread-id suffix is required because `cargo test` runs in parallel — a same-pid temp path races across threads.

### MCP server (`bin/memdeck-mcp`)

Pure C, links the engine sources directly, speaks JSON-RPC 2.0 over stdio using vendored yyjson (`mcp/vendor/yyjson/`, MIT, 0.10.0). Seven tools exposed for ABC-songwriting workflows (inspect, render stats, validate, list demos, directive help, duration calc, engine caps). See `mcp/README.md` for the registration command and tool table.

Subtle yyjson gotcha embedded in that README: `yyjson_mut_obj_add_str` stores the value pointer **without** copying. Any non-`'static` string — especially loop-local stack buffers — must use `add_strcpy` or the response gets garbled. The bug pattern shows up as truncated/null-padded values that all reference the last loop iteration.

## Code style

- C99 with strict `-Wall -Wextra` (yyjson is compiled separately under `-w`).
- Integer fixed-point math (Q15) everywhere in DSP — no floats in the render path. Determinism matters more than the last 1 dB of analog warmth.
- Avoid feature flags / backwards-compat shims for in-tree changes; the only consumers of an internal API live in this repo.
- Comments should explain *why* (non-obvious constraints, references to papers/ADRs), not *what*.

## Things to read before touching audio

- `docs/audio-architecture.md` — overall pipeline
- `docs/audio-abc-dsl.md` — the `%%` directive vocabulary
- `docs/audio-fx.md` — FX bus stages and their parameters
- `docs/adr-0001-gui-direct-audio-playback.md` — why GUI playback is `cpal`-side, not C
- `docs/adr-0002-moog-ladder-filter.md` — why the ladder is pure C in-house
- `docs/composer-guide.md` — practical guidance for writing showcase songs
