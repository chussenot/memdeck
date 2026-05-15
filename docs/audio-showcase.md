# Audio Showcase

Use `make render-demos` to render the showcase catalog through the public audio engine API.

Use `make play-demo DEMO=name` for single-track iteration.

Optional WAV export for sharing/listening:

- `make play-demo DEMO=neon_nightdrive WAV=1`
- writes `bin/wav/neon_nightdrive.wav`

Each render reports:

- title
- duration
- PCM length
- checksum
- clipping count

## Current deterministic outputs

| Title | Duration | PCM length | Checksum | Clipping |
| --- | ---: | ---: | --- | ---: |
| Dark Moroder - Synth Disco | 15.484s | 341419 | `0x51c34a9304aa4556` | 2187 |
| Perturbator Loop - Dark Synth | 13.521s | 298140 | `0xe610f99b4c4b6b37` | 4656 |
| Carpenter Drive - Horror Synth | 18.461s | 407076 | `0xaab706f01eaac6ea` | 3803 |
| Advanced DSL Demo - Instruments and FX Buses | 15.000s | 330750 | `0x064097bd78e56a0d` | 218 |
| Multi-FX Bus Demo - Dual Processing | 14.328s | 315940 | `0xc461e95426b0fd72` | 798 |
| Neon Nightdrive - Hypnotic Arp | 15.738s | 347016 | `0x3cae36a6a440ea7d` | 188 |
| Metro Chase - Aggressive Electro | 13.151s | 289972 | `0x129a5338dc6a44c9` | 13564 |
| Black Sunrise - Cinematic Arps | 17.143s | 378000 | `0x5c427d0791f0c53c` | 681 |
| Machine Romance - Disco Pulse | 16.000s | 352800 | `0x39615b7ff5900283` | 3757 |
| Hypersleep Dream - Atmospheric Drift | 20.000s | 441000 | `0xb6a2b00a57398405` | 84 |

## Writing stable demos

- keep timing deterministic with explicit `Q:`, `L:`, and `%%swing`
- define named instruments and route each voice with explicit `fx=` bus assignment
- use `%%pattern` and `%%arrangement` to keep authored structure visible
- use section comments (`% section intro`, `% section bridge`) for composer readability
- keep clipping controlled to preserve groove and avoid brittle harshness
