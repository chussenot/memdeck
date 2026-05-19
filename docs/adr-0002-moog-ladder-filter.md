# ADR-0002: Moog ladder filter as in-house C DSP

- Status: Accepted
- Date: 2026-05-19
- Deciders: MemDeck maintainers

## Context

MemDeck's FX bus chain (`drive → lowpass → delay → sidechain`) uses a one-pole IIR lowpass. That gives a 6 dB/oct slope with no resonance — fine for gentle tone shaping but unable to produce the resonant, self-oscillating quack that defines analog-modeled synthwave/grunge tones.

We want a Moog-style 4-pole (24 dB/oct) ladder filter with resonance and soft-clip non-linearity as a new FX bus stage, so songs can opt into it via the `%%effect N ladder=… ladder_cutoff=… ladder_resonance=…` directive.

Goal: a filter that produces audibly Moog-flavored output, fits the existing render pipeline without forking it, and stays deterministic across all consumers (CLI tools, `make test`, GUI playback).

## Options scored

Scores: 1 (worst) to 5 (best).

| Option | Render-path consistency | Integration effort | Build / license risk | Maintenance | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| **Pure C in-house** (transcribed Stilson-Smith / Huovilainen, Q15 integer math, in `audio_fx.c`) | 5 | 4 | 5 | 4 | 18 |
| C++ vendored (`ddiakopoulos/MoogLadders`, single-file Apache-2.0) | 5 | 2 | 3 | 3 | 13 |
| Rust crate (`surgefilter-moog` / `fundsp` / `synfx-dsp`) in GUI only | 2 | 3 | 4 | 4 | 13 |
| Faust `ve.moogLadder` exported to C/C++ | 5 | 2 | 4 | 2 | 13 |

### Why the scores

**Pure C in-house (18 / 20)**

- *Render consistency 5*: lives in the shared C engine; CLI `play-demo`, `render-demos`, `make test`, the showcase regression goldens, and GUI playback all hit the same code.
- *Integration 4*: drops in next to `AudioLowpass`/`AudioDelay`/`AudioSidechain` with the same init/free/process shape. New `SeqFxBus`/`AbcFxBus` fields and a few lines of parser glue.
- *Build/license 5*: no new dependency, no language mix, no upstream to track. The algorithm is textbook; transcribed from public references with attribution in comments.
- *Maintenance 4*: ~100 LOC of straightforward fixed-point math. Q15 conventions already established in the file.

**C++ vendored MoogLadders (13 / 20)**

- *Render consistency 5*: same shared C engine path.
- *Integration 2*: forces enabling C++ in `Makefile` and `gui/build.rs`, plus name-mangling/extern "C" wrappers. MoogLadders uses `float` and STL — converting to integer math defeats the point of vendoring.
- *Build/license 3*: Apache-2.0 is permissible but adds NOTICE-tracking and a C++ compile dependency.
- *Maintenance 3*: keeping a vendored upstream in sync invites drift; without it we own the fork anyway, so the vendoring buys little.

**Rust crate in GUI only (13 / 20)**

- *Render consistency 2*: splits the render path. CLI tools and `tests/*.c` keep the old (no-ladder) sound; only GUI playback would change. Showcase goldens diverge between paths. This is the disqualifier.
- *Integration 3*: easy to add to the GUI crate, hard to expose back to the C engine without an FFI shim going the wrong direction.
- *Build/license 4*: another cargo dep, MIT/Apache typical.
- *Maintenance 4*: well-maintained upstreams, but the path-split debt is permanent.

**Faust-generated (13 / 20)**

- *Render consistency 5*: emit C, link into the engine.
- *Integration 2*: requires Faust toolchain to regenerate; non-Faust contributors can't tweak the filter without re-deriving from `.dsp`.
- *Build/license 4*: emitted code is BSD-ish, but the build dependency is heavy.
- *Maintenance 2*: generated C is opaque and unidiomatic. Any bugfix has to round-trip through Faust source.

## Decision

Implement a **pure C Moog ladder filter** in `src/audio_fx.{c,h}` as a new FX bus stage. Use Q15 integer fixed-point math consistent with the rest of the file. Place it in the chain between `lowpass` and `delay`, so the existing one-pole stays available for gentle tone shaping while the ladder handles resonant character.

## Rationale

- The killer constraint is render-path consistency. The showcase regression tests (`tests/test_abc.c`) lock in bit-exact PCM checksums; any DSP that only runs in the GUI breaks that contract. Only the three in-engine options qualify, and of those the pure-C path has the lowest integration cost and zero new build/license surface.
- cpal (ADR-0001) is for audio I/O, not DSP — the Moog character has to come from the engine, not from the playback backend. This ADR is the DSP-layer companion to that decision.
- The algorithm itself is well-described in the literature (Stilson 1996, Huovilainen 2004, Pirkle DAFX-19). Transcribing it into our Q15 conventions is straightforward and review-friendly.

## Consequences

Positive:

- ladder is available to every consumer of `audio_engine_render_abc_file` automatically
- no new language, build tool, or third-party dependency
- existing songs render bit-identical until they opt in via `ladder=…`
- algorithm is in-tree and reviewable, not behind a crate boundary

Trade-offs:

- we own the implementation; subtle differences between our transcription and the canonical floating-point model are our problem
- integer fixed-point trades some analog warmth for determinism (no FP rounding drift across platforms — a feature for our test suite, a limit for tone)
- high-resonance self-oscillation needs explicit clamping to avoid Q15 overflow

## Implementation notes

1. New type `AudioMoogLadder` in `audio_fx.h` with four Q15 stage states, an `alpha_q15` cutoff coefficient, and a `feedback_q15` resonance gain.
2. `audio_fx_moog_ladder_init(rev, sample_rate, cutoff_percent, resonance_percent)` computes coefficients from 0–100 user parameters.
3. `audio_fx_moog_ladder_process(rev, input)`: subtract `(stage[3] * feedback_q15) >> 15` from the input, soft-clip, then cascade four one-pole sections.
4. Soft-clip is a cheap polynomial (`x − x³/3` family) clamped to the existing `FX_BUFFER_CLAMP`, not a full `tanh`.
5. New `SeqFxBus` fields: `ladder_amount`, `ladder_cutoff`, `ladder_resonance`. New `AbcFxBus` fields and the Rust mirror in `gui/src/ffi.rs` and `gui/src/editor/model.rs`. Parser reads `ladder=` / `ladder_cutoff=` / `ladder_resonance=` from `%%effect`.
6. Chain order in `audio_fx_bus_process`: `drive → lowpass → ladder → delay → sidechain → mix`. With `ladder_amount=0` the stage short-circuits to bit-identical output.
7. Smoke-render every showcase demo before opting any song in; only after the no-op pass is green do we add `ladder=` directives to the songs that want the character.

## Validation

- `make test` passes with no demo opting in (every existing PCM checksum bit-identical).
- A small selection of demos opt in (initial: `dark_moroder`, `perturbator_loop`, `glass_anthem`, `surrender_loop`), and their showcase goldens are refreshed in the same commit that flips them on.
- Audible spot-check via `make play-demo DEMO=<name>` for each opted-in song.
