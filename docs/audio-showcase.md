# Audio Showcase

Use `make render-demos` to render the demo catalog through the public audio engine API.

The command prints:

- title
- duration
- PCM length
- checksum
- clipping count

## Current deterministic outputs

| Title | Duration | PCM length | Checksum | Clipping |
| --- | ---: | ---: | --- | ---: |
| Dark Moroder - Synth Disco | 7.619s | 168000 | `0x5d483689cfbba730` | 1821 |
| Perturbator Loop - Dark Synth | 6.857s | 151200 | `0x778fd5f57cf83b36` | 5881 |
| Carpenter Drive - Horror Synth | 9.412s | 207529 | `0xa739faef6dac5b67` | 4009 |
| Advanced DSL Demo - Instruments and FX Buses | 17.308s | 381634 | `0xe74909ce1c1beae3` | 2153 |
| Multi-FX Bus Demo - Dual Processing | 14.222s | 313600 | `0x25da3aed91b9d649` | 12436 |

## Writing stable demos

- keep timing deterministic
- use named instruments and explicit FX bus routing
- use `%%pattern` and `%%arrangement` for intentional structure
- keep clipping controlled enough for regression thresholds
