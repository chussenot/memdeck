# Audio Showcase Tracks

The showcase demos are:

- `data/music/dark_moroder.abc`
- `data/music/perturbator_loop.abc`
- `data/music/carpenter_drive.abc`
- `data/music/advanced_dsl_demo.abc`
- `data/music/multi_fx_demo.abc`

## Render and inspect

Run:

```sh
make render-demos
```

The command renders each track and prints:

- file name
- duration
- PCM length
- checksum
- peak/min/max
- clipping count

## Writing showcase-quality demos

- Use named `%%instrument` presets for reusable timbre/ADSR design.
- Use `%%effect` buses to separate punchy and ambient roles.
- Use `%%pattern` and `%%arrangement` for intentional structure.
- Use accents/transients so sidechain pumping is audible but controlled.
- Keep output deterministic so regression checks remain stable.
