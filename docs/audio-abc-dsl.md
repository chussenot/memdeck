# MemDeck Audio ABC DSL

MemDeck extends ABC notation so tracks can be parsed into `SeqSong` and rendered by the shared sequencer/mixer/FX pipeline.

## Core mapping

- `Q:` -> `SeqSong.tempo_bpm`
- `%%swing` -> `SeqSong.swing_pct`
- `%%instrument` -> `SeqInstrument`
- `%%effect` / `%%sidechain` -> `SeqFxBus`
- `%%pattern` -> `SeqPattern`
- `%%arrangement` -> `SeqSong.arrangement`
- `V:` voices -> `SeqTrack`
- notes/rests -> `SeqStep`

## Supported directives

- `%%swing <value>`
- `%%instrument <name> ...`
- `%%effect ...` (legacy single-bus form)
- `%%effect <bus> ...` (numbered bus form)
- `%%sidechain ...`
- `%%pattern <name> length=<steps>`
- `%%arrangement <pattern-name> ...`
- `%%voice <name> ...`

## Instrument example

```abc
%%instrument bassline wave=pulse amp=88 duty=32 attack=1 decay=45 sustain=72 release=90 gate=78 fx=0
V:bass instrument=bassline
```

## FX examples

Single bus (legacy-compatible):

```abc
%%effect delay time=3 feedback=35 mix=25
%%effect drive amount=20
%%effect lowpass amount=30
%%sidechain amount=40 release=180
```

Numbered buses:

```abc
%%effect 0 delay_steps=1 delay_feedback=18 delay_mix=10 drive=24 lowpass=26 sidechain=46 sidechain_release=180 mix=100
%%effect 1 delay_steps=4 delay_feedback=38 delay_mix=34 drive=8 lowpass=38 sidechain=32 sidechain_release=210 mix=78
```

## Arrangement example

```abc
%%pattern A length=16
%%pattern B length=16
%%arrangement A A B A
```

## Composition notes

- Keep directives deterministic (no randomness in parser or renderer).
- Prefer explicit instruments and FX bus routing for intentional mix results.
- Keep `L:` and `Q:` musically consistent so note timing maps cleanly to sequencer steps.
