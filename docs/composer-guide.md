# Composer Guide

This guide focuses on authored retro-synth composition with the MemDeck ABC DSL.

## Composition workflow

1. Start with groove: kick/noise pulse + bassline call/response.
2. Add one arpeggio lane for motion.
3. Add one pad/texture lane for harmonic glue.
4. Reserve lead/motif entries for section transitions.
5. Structure the file with section markers and patterns.

### Recommended section markers

- `% section intro`
- `% section verse`
- `% section bridge`
- `% section climax`
- `% section outro`

## Reusable preset families

Use named `%%instrument` presets and copy these families between tracks.

| Family | Preset(s) | Character | BPM range | FX guidance |
| --- | --- | --- | --- | --- |
| Bass | `heavy_bass` | Pulse, medium duty, fast attack, short release; mechanical disco drive and sidechain-friendly low end | 110-150 | Moderate drive + light delay + sidechain 40-60 |
| Arp | `plucky_arp` | Pulse, short decay/release, lower gate, optional vibrato; hypnotic movement without smearing | 110-150 | Medium delay bus, lower drive |
| Pad | `soft_pad` | Triangle, long attack/release, high sustain; cinematic bed and tension support | 90-135 | Wet delay bus, stronger lowpass |
| Lead | `synth_lead` | Pulse/triangle, moderate attack, glide + vibrato; emotional hooks and section punctuation | 100-145 | Controlled delay mix; keep bus mostly dry in dense mixes |
| Percussion | `solid_kick`, `hats_noise` | Noise, short envelope/gate; analog-machine pulse and groove articulation | 95-155 | Mostly dry; keep kick bus highly sidechain-relevant |

## Writing better basslines

- Alternate root and fifth for momentum.
- Introduce one chromatic or modal approach note before key changes.
- Use octave drops sparingly in bridges/climaxes.
- Keep bass rhythm locked to kick accents before adding complexity.

## Writing stronger arpeggios

- Cycle chord tones in 4-note cells, then vary one tone each section.
- Use register changes in bridge/climax for lift.
- Keep arp gate shorter than bass gate so both remain readable.

## Emulating analog feel

- Use pulse duty variation across lanes.
- Add subtle vibrato to moving parts only.
- Use glide mainly on lead hooks, not everywhere.
- Keep slight swing (`52-60`) for mechanical-but-human motion.

## Delay usage

- Use short delay on punch bus; long delay on ambient bus.
- Keep feedback moderate to avoid transient washout.
- Reduce delay mix when lead and arp are both busy.

## Sidechain pumping

- Put kick + bass on bus 0 with sidechain enabled.
- Keep sidechain amount high enough to breathe but not collapse sustain.
- Adjust release by tempo: faster BPM = shorter release.

## Creating tension and release

- Tension: denser arp, reduced bass sustain, dissonant color notes.
- Release: return to root-focused bass and wider pad intervals.
- Use motif silence before climactic entries.

## Avoiding muddy low-res mixes

- Avoid stacking too many low-mid voices.
- Keep pad amplitude lower than bass/arp anchors.
- Prefer arrangement-level variation over louder everything.
- If clipping spikes, lower bass amp or drive first.

## Composing for 8-bit style PCM

- Prioritize rhythmic clarity over dense harmony.
- Write memorable motifs with clear contour.
- Treat noise percussion as timing glue.
- Validate often with `make play-demo` and `make render-demos`.
