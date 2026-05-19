# ADR-0001: Direct GUI audio playback without external programs

- Status: Proposed
- Date: 2026-05-19
- Deciders: MemDeck maintainers
- Related: `gui/src/playback.rs`, `docs/gui-runtime.md`, `docs/gui-ffi.md`

## Context

The GUI currently renders PCM from the C engine, writes a temporary WAV file, and delegates playback to OS commands (`aplay`, `afplay`, PowerShell `SoundPlayer`).

This adds avoidable process and filesystem overhead, creates platform/tooling dependency drift, and makes playback behavior harder to keep deterministic across environments.

We want direct in-process playback while keeping the current render path (`EditableSong -> ABC -> C engine -> PCM`) intact.

## Decision drivers

- Remove dependency on third-party host playback programs
- Lower playback start latency and jitter
- Keep cross-platform support (Linux/macOS/Windows)
- Minimize unsafe/FFI complexity and long-term maintenance risk
- Keep CI and local developer setup simple

## Options considered

1. **Keep current external command approach** (baseline)
2. **C-level backend**: embed `miniaudio` in C and expose playback controls through FFI
3. **Rust-level backend (high-level)**: `rodio` (on top of `cpal`)
4. **Rust-level backend (low-level)**: `cpal` direct output stream with ring buffer

## Benchmark matrix (decision benchmark)

Scored 1 (worst) to 5 (best), weighted toward runtime behavior and maintainability.

| Criterion | Weight | External command | C `miniaudio` | Rust `rodio` | Rust `cpal` |
|---|---:|---:|---:|---:|---:|
| Startup latency path length (spawn/filesystem/device init) | 0.25 | 1 | 4 | 4 | 5 |
| Runtime stability/jitter control | 0.20 | 2 | 4 | 4 | 5 |
| Cross-platform parity | 0.15 | 2 | 4 | 4 | 4 |
| Integration complexity in current GUI | 0.15 | 5 | 2 | 4 | 3 |
| Safety/debuggability | 0.10 | 3 | 2 | 4 | 4 |
| CI/developer ergonomics | 0.10 | 2 | 3 | 4 | 4 |
| Future extensibility (seeking, scrubbing, preview tails) | 0.05 | 2 | 4 | 4 | 5 |
| **Weighted score** | **1.00** | **2.10** | **3.45** | **4.00** | **4.35** |

## Decision

Adopt a **Rust-level direct playback backend using `cpal`** in the GUI crate, with a small abstraction layer in `gui/src/playback.rs`.

### Why this is the best fit

- Best weighted benchmark result for this repository’s constraints
- Removes external playback program dependency completely
- Avoids introducing new C-side audio runtime/FFI ownership complexity
- Keeps audio render source-of-truth in existing C engine while moving playback control to safe Rust
- Supports future waveform scrubbing and low-latency start/stop behaviors needed by the GUI

## Consequences

### Positive

- No temporary WAV file required in steady-state playback
- More predictable playback startup and control path across platforms
- Cleaner error model in Rust UI state (no child process polling/exit-code mapping)

### Trade-offs

- Adds a new Rust dependency (`cpal`) and stream lifecycle management complexity
- Requires explicit sample-format conversion (`u8` mono buffer to backend format)
- Device enumeration differences across hosts must be handled robustly

## Implementation notes

1. Add a backend trait in `gui/src/playback.rs` and keep current command backend as fallback feature during migration.
2. Implement `cpal` backend:
   - single output stream
   - lock-free/ring-buffer feed from rendered PCM
   - start/stop/seek from GUI progress model
3. Convert mono 8-bit PCM to device sample format (`f32` preferred internal path).
4. Remove temp WAV + command spawn path after parity validation.
5. Keep optional WAV export only for debugging (not runtime playback path).

## Validation/benchmark plan for rollout

Measure on Linux/macOS/Windows in identical test scenarios:

- Playback start latency (Space press -> first sample submitted)
- Start/stop reliability over 200 rapid toggles
- Seek restart latency at 0%, 50%, 90%
- XRuns/dropouts over 3-minute continuous playback
- CPU usage during playback (idle GUI + active GUI interactions)

Adoption gate:

- Startup latency median improves vs baseline external-command path
- No regressions in render determinism (render path remains unchanged)
- No command-line audio tool requirement documented for GUI playback

