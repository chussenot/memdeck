# Audio architecture notes

## Goals
- Keep portable C99/C11 code for Linux + Emscripten.
- Keep retro/chiptune character (square/pulse-centric voices).
- Reduce CPU cost in hot loops.
- Keep deterministic timing and predictable buffers.

## Current structure
- `src/sound.c` = native backend orchestration (`fork`, `pipe`, `aplay`) + hardcoded fallback music.
- `src/abc.c` = ABC parsing and PCM rendering.
- `src/audio_dsp.c/.h` = platform-neutral DSP core used by both renderers.
- Naming stays as `audio_dsp` for now to avoid broad file churn, but it remains the portable audio core layer.

## DSP core (`audio_dsp`)
- **Oscillator abstraction**: `DspOscillator` with waveforms:
  - square
  - pulse
  - triangle
  - noise
- **Phase accumulator model**:
  - `phase` + `increment` in 32-bit fixed-point domain.
  - Frequency cost paid once per note/voice setup.
  - Sample generation in hot loop is integer stepping only.
- **Timing model**:
  - `DspSampleStepper` distributes fractional samples across steps.
  - Avoids cumulative drift from integer truncation (`sample_rate * ms / 1000`).
- **Mixing model**:
  - mono U8 centered at 128.
  - sum signed voice contributions then clamp 0..255.

## Buffer model
- SFX (`sound_success` / `sound_fail`) now use fixed stack buffers (no heap alloc per trigger).
- Music/ABC loop render keeps one contiguous heap PCM block for repeated writeout.
- Pipe writes handle partial writes in loops for robustness.

## Profiling hooks
- `SoundProfile` API:
  - `sound_profile_reset()`
  - `sound_profile_snapshot()`
- Counters include generated samples/calls/time, estimated loop latency, underrun events.
- Hooks are lightweight; generation timing uses `clock()` ticks.

## ABC voice extensions
- `V:` / `%%voice` directives now accept:
  - `wave=square|pulse|triangle|noise`
  - `duty=N` (pulse duty cycle, 1..99)
- Defaults remain square-like to preserve project sound identity.

## Render-path optimization
- `abc_generate_pcm()` now precompiles a lightweight per-voice step timeline before mixing.
- Consecutive identical notes reuse oscillator state instead of reinitializing on every step.
- `sound.c` applies the same repeated-note reuse for the hardcoded fallback loop.

## Portability strategy
- Keep DSP logic in `audio_dsp` free of platform I/O APIs.
- Keep backend-specific output in `sound.c` (Linux process/audio piping today).
- This split allows future wasm/native output backends without changing oscillator/mixer kernels.

## Verification
- Native verification chain remains:
  - `make clean`
  - `make all`
  - `make test`
  - `make bench-audio`
- WASM verification now includes:
  - `make -C wasm verify`
  - `node wasm/verify-audio.js`
  - `make -C wasm`

## Future SIMD/WASM-ready direction
- `dsp_osc_next()` and per-step mix loops are isolated kernels.
- Next optimization step can batch voice generation to arrays for vectorized mixing.
- No assembly required; portable intrinsics can be added later behind compile-time flags.
