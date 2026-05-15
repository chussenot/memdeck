# Composer Guide

This guide focuses on authored retro-synth composition using the MemDeck ABC DSL.

## Composition workflow

1. Start with groove: kick/noise pulse + bassline call/response.
2. Add one arpeggio lane for motion.
3. Add one pad/texture lane for harmonic glue.
4. Reserve lead/motif entries for section transitions.
5. Structure the file with section markers and patterns.

Recommended section markers:

- `% section intro`
- `% section verse`
- `% section bridge`
- `% section climax`
- `% section outro`

## Reusable preset families

Use named `%%instrument` presets and copy these families between tracks.

### Bass presets

- **heavy_bass**: pulse, medium duty, fast attack, short release, fx bus 0
- intent: mechanical disco drive and sidechain-friendly low end
- BPM range: 110-150
- FX: moderate drive + light delay + sidechain 40-60

### Arp presets

- **plucky_arp**: pulse, short decay/release, lower gate, optional vibrato
- intent: hypnotic movement without smearing
- BPM range: 110-150
- FX: medium delay bus, lower drive

### Pad presets

- **soft_pad**: triangle, long attack/release, high sustain
- intent: cinematic bed and tension support
- BPM range: 90-135
- FX: wet delay bus, stronger lowpass

### Lead presets

- **synth_lead**: pulse/triangle, moderate attack, glide + vibrato
- intent: emotional hooks and section punctuation
- BPM range: 100-145
- FX: controlled delay mix, keep bus mostly dry if mix is dense

### Percussion presets

- **solid_kick / hats_noise**: noise, short envelope, short gate
- intent: analog-machine pulse and groove articulation
- BPM range: 95-155
- FX: mostly dry, high sidechain relevance on kick bus

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
- Keep slight swing (52-60) for mechanical-but-human motion.

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
