# MemDeck GUI Pattern View

The pattern/arrangement overview is a read-only, tracker-inspired visualization of the step activity across all tracks in the selected demo.

## Layout

```text
+--------------------+------------------------------------+
| TRACK 01 / inst    | [P1     ][P2     ][P3     ][P4   ] |
| TRACK 02 / inst    | [  ][  ][  ][  ][  ][  ][  ][    ] |
| TRACK 03 / inst    | [ ][  ][ ][ ][  ][ ][ ][  ][ ][  ] |
| TRACK 04 / inst    | [    ][      ][    ][      ][      ] |
+--------------------+------------------------------------+
```

The view is split into two horizontal zones:

- **Label column** (left, fixed width): shows `track_name / instrument_ref` for each track.
- **Timeline column** (right): shows arrangement blocks and step-level activity dots.

## Arrangement Header

The top strip of the timeline column shows the arrangement blocks defined in the ABC file:

- Each block represents a named pattern segment (e.g. `Verse`, `Chorus`, `P1`).
- Block boundaries are derived from `%%pattern` / `%%arrangement` directives, or auto-generated 16-step chunks if no named patterns exist.
- Block labels are rendered in `ACCENT` green at the block center.
- Adjacent blocks are separated by `BORDER_DIM` strokes.

## Track Rows

Each row corresponds to one sequencer voice (up to 4 visible; additional voices are counted but not rendered).

- Alternating row fills (`PANEL_DIM_BG` / `PANEL_BG`) improve scanability.
- A horizontal `GRID` stroke separates each row.
- A vertical `BORDER_DIM` stroke at each block boundary aligns rows with the arrangement header.

## Step Activity Cells

Each active step (note frequency > 0 in the rendered ABC voice) is drawn as a filled cell:

- Fill: `ACCENT_SOFT`
- Stroke: `ACCENT` (1 px)
- Cell height: row height minus 8 px padding top/bottom.
- Cell width: proportional to one step within the total step count.

Inactive steps (rest / frequency = 0) have no cell — the row background is visible.

## Empty / Unavailable State

| Condition | Display |
|---|---|
| No overview loaded | `PATTERN DATA NOT AVAILABLE FOR THIS DEMO.` (WARNING color) |
| Overview present but no arrangement or tracks | `NO PATTERN ARRANGEMENT FOUND.` (TEXT_DIM color) |

## Data Source

Pattern data is extracted from the ABC file at demo load time by `ffi::load_demo_overview()` → `build_demo_overview()`. It is stored in `DemoOverview`:

```
DemoOverview {
  arrangement: Vec<PatternBlock>,   // label, start_step, length
  tracks: Vec<TrackOverview>,       // name, instrument, activity: Vec<bool>
  total_steps: usize,
  hidden_track_count: usize,        // voices beyond SEQ_MAX_TRACKS (4)
}
```

## Design Constraints

- Read-only. No step editing.
- No zoom or scroll. The full arrangement fits in the available width.
- Maximum 4 visible tracks (seq engine hard limit).
- No piano-roll, no note labels, no timing grid numbers.
