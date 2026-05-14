# Audio optimization report

## Implemented optimizations

1. Hot-loop floating point removal
- Replaced per-sample `i / half_period` + modulo logic with fixed-point phase accumulators.
- Frequency-to-increment conversion is done once per oscillator setup.

2. Oscillator abstraction
- Added reusable oscillator core (`src/audio_dsp.c/.h`).
- Supports square, pulse, triangle, noise.

3. Deterministic timing stability
- Introduced `DspSampleStepper` to distribute fractional samples across steps.
- Removes drift in both hardcoded music and ABC rendering.

4. MIDI frequency lookup table
- Replaced per-note `pow()` calls in `semitone_to_freq()` with a precomputed
  128-entry `midi_freq_table[]` (MIDI notes 0–127).
- `<math.h>` removed from `abc.c`; no floating-point transcendentals remain
  in the parsing or render hot paths.

5. Buffer/allocation improvements
- SFX now use stack buffers (removed per-trigger malloc/free).
- PCM generation remains contiguous and cache-friendly.
- Pipe writes now correctly handle partial writes.

5. Profiling hooks
- Added optional sound profile snapshot API for generated sample/time stats and underrun accounting.

6. Architecture cleanup
- Shared DSP moved into isolated portable module consumed by `sound.c` and `abc.c`.

7. ABC rendering improvements
- PCM generation uses per-voice oscillator state arrays per step.
- Added lightweight waveform directives for future extension while preserving defaults.

## Deterministic regression tests

- `tests/test_audio_dsp.c` exercises every public DSP function with exact
  integer expected values: clamp, sample helpers, oscillator phase transitions
  for square and pulse waveforms, triangle wave at phase 0, noise LFSR
  variation, stepper distribution, and stepper-sum identity.
- `tests/test_abc.c` validates ABC parser + PCM generation end-to-end for
  both shipped music tracks.
- Both run as part of `make test`. Neither requires external dependencies.

## Benchmarks
Environment: Linux native build, `-O2`, `bin/bench-audio` (`make bench-audio`).

Representative run (values vary by hardware/load):
- oscillator: ~72 ms  (22050 × 2000 = 44.1 M samples)
- mix loop:   ~2050 ms (3-channel mix, 64 steps × 2000 iterations)
- stepper:    ~378 ms  (stepper only, 64 steps × 2 000 000 iterations)

## Build target summary

| Target | Action |
|---|---|
| `make all` | Native TUI binary (includes `audio_dsp.c`) |
| `make test` | Runs all tests including `test-audio` and `test-abc` |
| `make test-audio` | Builds and runs `bin/test-audio-dsp` (DSP regression) |
| `make test-abc` | Builds and runs `bin/test-abc` (ABC parser + PCM) |
| `make bench-audio` | Builds and runs `bin/bench-audio` (microbenchmark) |
| `make -C wasm` | WASM build — `audio_dsp.c` is intentionally excluded; Web Audio API handles sound in the browser |


- Maintains retro/chiptune character (square-first synthesis, pulse arp flavor).
- Music loading behavior and fallback flow are unchanged.
- Native tests remain passing.

## Known constraints
- Writer-side underrun reporting is coarse (pipe failure based) by design to stay lightweight.
- WASM build validation requires `emcc` toolchain in environment.
