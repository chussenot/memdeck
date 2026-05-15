# MemDeck Audio ABC DSL

MemDeck parses ABC into `AbcMusic`, converts that into `SeqSong` with `abc_build_seq_song()`, and renders through the shared sequencer/mixer/FX path.

## Supported directives

- `%%swing <value>`
- `%%instrument <name> ...`
- `%%effect ...` (legacy single-bus form)
- `%%effect <bus> ...` (numbered bus form)
- `%%sidechain ...`
- `%%pattern <name> length=<steps>`
- `%%arrangement <pattern-name> ...`
- `%%voice <name> ...`

## SeqSong mapping

- `Q:` -> `SeqSong.tempo_bpm`
- `%%swing` -> `SeqSong.swing_pct`
- `%%instrument` / `V: ... instrument=...` -> `SeqInstrument`
- `%%effect` / `%%sidechain` -> `SeqFxBus`
- `V:` sections -> `SeqTrack`
- notes/rests -> `SeqStep.note`
- voice gate/staccato defaults -> `SeqStep.gate`
- deterministic default velocity -> `SeqStep.velocity`
- first step of each beat -> `SeqStep.accent` and `SeqStep.fx_trigger`

## Patterns and arrangement

`%%pattern` stores named pattern lengths.

`%%arrangement` is used during `AbcMusic -> SeqSong` conversion to slice the sequential ABC note stream into ordered pattern instances. Each arrangement slot becomes one concrete `SeqPattern` in the rendered `SeqSong`, preserving the authored timeline while still using the arrangement metadata.

If an arrangement references an unknown pattern name, `abc_build_seq_song()` fails and rendering returns `NULL`.

## Legacy compatibility

Legacy ABC files without DSL directives still render:

- voices are mapped directly to tracks
- default FX bus 0 is available
- legacy `%%effect` / `%%sidechain` directives still map to bus 0

## Example

```abc
%%swing 56
%%instrument bassline wave=pulse amp=88 duty=32 attack=1 decay=45 sustain=72 release=90 gate=78 fx=0
%%effect 0 delay_steps=1 delay_feedback=18 delay_mix=10 drive=24 lowpass=26 sidechain=46 sidechain_release=180 mix=100
%%pattern A length=16
%%pattern B length=16
%%arrangement A A B A
V:bass instrument=bassline
```
