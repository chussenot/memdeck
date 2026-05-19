# ADR-0001: Direct in-process audio playback for GUI rendered tracks

- Status: Accepted
- Date: 2026-05-19
- Deciders: MemDeck maintainers

## Context

The GUI currently renders PCM, writes a temporary WAV file, then spawns a platform program (`afplay`, `aplay`, or PowerShell `SoundPlayer`) to hear the track.

That path works, but it adds:

- external runtime dependency on host audio programs
- process lifecycle complexity
- temp-file I/O and cleanup paths
- playback behavior variance across platforms

Goal: play rendered tracks directly in-process from the existing PCM buffer, without shelling out to external programs.

## Options benchmarked

We benchmarked options against MemDeck constraints (GUI-first change, cross-platform behavior, low maintenance risk, deterministic render path unchanged).

Scores: 1 (worst) to 5 (best).

| Option | Runtime overhead | Integration effort | Cross-platform packaging | Maintenance risk | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| Keep current WAV + external player | 2 | 5 | 2 | 2 | 11 |
| C-level backend (`miniaudio` via FFI) | 4 | 2 | 4 | 3 | 13 |
| Rust-level backend (`rodio`) | 4 | 4 | 4 | 4 | 16 |
| Rust-level backend (`cpal`) | 5 | 3 | 4 | 4 | 16 |

### Measured baseline (current WAV handoff)

Local write benchmark on this branch for U8 mono 22,050 Hz WAV output (200 iterations, tmpfs-backed environment):

- 30s buffer: median 0.534 ms, p95 0.643 ms
- 60s buffer: median 0.812 ms, p95 33.245 ms
- 180s buffer: median 2.024 ms, p95 39.564 ms

Notes:

- This captures temp WAV write cost only.
- It does not include external player startup latency (not reproducible in this sandbox due to missing host player binaries/devices).
- Tail jitter confirms file/process handoff variability that an in-process stream can remove.

## Decision

Adopt a **Rust-level direct playback backend** in `gui/`, implemented with **`cpal`** (with a small GUI-local buffer/queue), and stop using temporary WAV + external playback commands for the GUI path.

## Rationale

- Scope fit: this problem is in the Rust GUI runtime (`gui/src/playback.rs`), so a Rust backend keeps changes local.
- No extra C/FFI surface: avoids introducing new C playback APIs and cross-language ownership/lifecycle complexity.
- Better control: `cpal` gives explicit stream callback control for play/stop/seek/progress behavior.
- Future-proofing: if a higher-level API is preferred later, `rodio` can still be layered on top of `cpal` patterns.

## Consequences

Positive:

- no required `afplay`/`aplay`/PowerShell audio program for GUI playback
- no temp WAV file creation/cleanup in normal GUI playback
- lower latency and less jitter risk on start/seek
- more consistent behavior across desktop platforms

Trade-offs:

- one new Rust audio dependency in the GUI crate
- callback/threaded audio code to maintain in Rust
- CI/headless environments may still need a no-audio fallback mode for smoke runs

## Implementation notes

1. Add a backend abstraction in `gui/src/playback.rs` (`ExternalCommandBackend` + `CpalBackend`).
2. Keep existing state/progress contract in `PlaybackController`.
3. Feed rendered PCM directly to the stream (convert U8 centered-at-128 to target sample type).
4. Keep external-command fallback behind a feature or runtime fallback path.
5. Remove temporary WAV ownership paths once `cpal` path is stable.

## Validation benchmark plan for implementation PR

After implementing the backend, collect side-by-side metrics for current vs new path:

- start latency: key press to first non-silent callback sample
- seek latency: click-to-audio after cursor seek
- stop latency: command-to-silence
- CPU cost: 60s playback of same rendered track
- glitch rate: underruns/callback starvation count

Accept if direct playback matches or improves current UX and removes external program dependency for GUI playback.
