# MemDeck GUI Design System

The visual system follows a strict retro Atari ST–inspired aesthetic. All rules are expressed as Rust constants in `gui/src/app.rs`.

## Color Palette

| Token | Value (RGB) | Usage |
|---|---|---|
| `BASE_BG` | `#0A0C0A` | Window and panel background fill |
| `PANEL_BG` | `#101410` | Primary panel fill |
| `PANEL_ALT_BG` | `#141A14` | Alternating row fill, arrangement block fill |
| `PANEL_DIM_BG` | `#0C100C` | Waveform / pattern canvas fill |
| `BORDER` | `#586A58` | Focused panel border, window stroke |
| `BORDER_DIM` | `#3A483A` | Unfocused panel border, grid dividers |
| `TEXT` | `#D6E2D6` | Primary labels, active hint text |
| `TEXT_DIM` | `#889688` | Secondary labels, field names, dim hints |
| `ACCENT` | `#82D690` | Focus highlight, active step stroke, play state |
| `ACCENT_SOFT` | `#486C4C` | Active step cell fill, selection background |
| `WARNING` | `#EA7A6A` | Clipping markers, errors, unavailable demos |
| `WAVEFORM` | `#C2DEC2` | PCM envelope lines |
| `GRID` | `#222C22` | Internal grid lines within canvases |

## Typography

All text uses **monospace only**. No proportional fonts.

| Style key | Font | Size | Usage |
|---|---|---|---|
| `Heading` | monospace | 21 pt | Panel header (`MEMDECK SOUND MACHINE`) |
| `Title` | monospace | 18 pt | (reserved) |
| `Body` | monospace | 14 pt | Demo list items, button labels, field values |
| `Button` | monospace | 14 pt | Key hint buttons |
| `Monospace` | monospace | 13 pt | Stats grid, section sub-labels |
| `Small` | monospace | 12 pt | Pattern block labels, track labels, footnotes |

Panel titles use **Body** (14 pt) with `.strong()` and `ACCENT` color when focused.

## Spacing

| Token | Value | Usage |
|---|---|---|
| `item_spacing` | 8 × 8 px | Default widget spacing |
| `button_padding` | 8 × 4 px | Button inner padding |
| `window_margin` | 10 px (all sides) | Panel inner margin |
| Panel inner margin | 10 px (all sides) | `retro_panel` frame |
| Panel title gap | 6 px | Space after panel title |
| Demo list row height | 22 px min | `min_size` on demo buttons |
| Stats grid column gap | 14 × 6 px | Grid spacing (label / value columns) |
| Waveform canvas height | 150 px | Fixed allocation |
| Waveform canvas shrink | 4 px | Canvas inset from allocated rect |
| Pattern row height | 24 px | Per-track row in pattern view |
| Pattern header height | 26 px | Arrangement block label strip |
| Label column width | 150–220 px | Track name area in pattern view |

## Borders and Focus

| State | Border width | Border color |
|---|---|---|
| Focused panel | 2 px | `ACCENT` |
| Unfocused panel | 1 px | `BORDER` |
| Canvas inset | 1 px | `BORDER_DIM` |
| Divider lines | 1 px | `BORDER_DIM` / `GRID` |

## Focus Model

Two keyboard focus areas exist:

| Area | Label shown in header |
|---|---|
| `FocusArea::DemoList` | `FOCUS  DEMOS` |
| `FocusArea::Overview` | `FOCUS  OVERVIEW` |

`Tab` cycles between the two. The focused panel's title and border are rendered in `ACCENT`; the unfocused panel uses `TEXT` and `BORDER`.

Both the `RENDER STATS` and `WAVEFORM / PATTERN OVERVIEW` panels belong to `FocusArea::Overview` — they share the same focus highlight because they form a single logical inspection region.

## No-Go Rules

- No gradients.
- No drop shadows.
- No animations or transitions.
- No rounded corners (corner radius = 0 on all canvases).
- No icons or images.
- No proportional fonts.
- No color outside the defined palette tokens.
