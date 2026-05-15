# MemDeck GUI Pattern View

The pattern overview is a read-only tracker-style map of arrangement and step activity.

## Layout

```text
+-----------------------------+---------------------------------------------+
| 01 KICK / kick              | A 16 | A 16 | B 16 | A 16 | ...         |
| 02 BASS / bassline          | beat grid + active cells + accent markers   |
| 03 ARP / arp16              | beat grid + active cells + FX markers       |
| 04 LEAD / leadpad           | selected row highlight + playback cursor    |
+-----------------------------+---------------------------------------------+
```

## What is shown

- left label column with track index, track name, and instrument name
- arrangement header using `PatternBlock { label, length, start_step }`
- beat grid derived from `DemoOverview.steps_per_beat`
- active steps from `TrackOverview.activity`
- accent emphasis using stronger fill
- FX trigger tick marks using the warning color
- selected-track row highlight for inspector coherence
- playback cursor when the render is playing

## Data model

`ffi::load_demo_overview()` builds:

```text
DemoOverview {
  steps_per_beat,
  total_steps,
  arrangement: Vec<PatternBlock>,
  tracks: Vec<TrackOverview>,
  fx_buses: Vec<FxBusOverview>,
}
```

Each `TrackOverview` contains `Vec<StepState>` where every step exposes:

- `active`
- `accent`
- `fx_trigger`

The current accent / FX markers are inferred from beat positions in the read-only ABC overview path, which keeps rendering lightweight and avoids promoting the screen into an editor.

## Constraints

- read-only only
- full arrangement fits in the panel width
- max 4 rendered rows (matching the sequencer display limit)
- no note names, piano roll, zoom, drag, or edit gestures
