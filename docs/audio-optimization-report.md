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

4. Buffer/allocation improvements
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

## Benchmarks
Environment: CI sandbox, native Linux build, `-O2`, benchmark binary `bin/bench-audio`.

Observed run:
- oscillator: `97191329 ns`
- mix loop: `2524899975 ns`
- stepper: `454424266 ns`

Notes:
- These are microbenchmark wallclock approximations from `clock()` tick conversion.
- Primary value is relative tracking across future revisions.

## Behavioral compatibility notes
- Maintains retro/chiptune character (square-first synthesis, pulse arp flavor).
- Music loading behavior and fallback flow are unchanged.
- Native tests remain passing.

## Known constraints
- Writer-side underrun reporting is coarse (pipe failure based) by design to stay lightweight.
- WASM build validation requires `emcc` toolchain in environment.
