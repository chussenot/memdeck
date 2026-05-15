# MemDeck GUI Design System

The GUI uses a compact retro-tool system tuned for Atari/ST-era workstation energy rather than modern DAW chrome.

## Color palette

| Token | Value | Use |
|---|---|---|
| `BASE_BG` | `#0A0C0A` | app background |
| `PANEL_BG` | `#101410` | main panel fill |
| `PANEL_ALT_BG` | `#141A14` | selected row / arrangement banding |
| `PANEL_DIM_BG` | `#0C100C` | scopes, meters, minimaps |
| `BORDER` | `#586A58` | panel frame |
| `BORDER_DIM` | `#3A483A` | inner dividers |
| `TEXT` | `#D6E2D6` | primary labels |
| `TEXT_DIM` | `#889688` | secondary labels |
| `ACCENT` | `#82D690` | focus, playhead, active state |
| `ACCENT_SOFT` | `#486C4C` | active cells / meter fill |
| `ACCENT_DIM` | `#345038` | accent-step emphasis |
| `WARNING` | `#EA7A6A` | errors, clipping, hot FX |
| `WAVEFORM` | `#C2DEC2` | waveform stroke |
| `GRID` | `#222C22` | beat grid |

## Typography

All visible text is monospace.

| Style | Size | Use |
|---|---|---|
| Heading | 21 pt | window title |
| Title | 18 pt | reserved |
| Body / Button | 14 pt | panel titles, list rows, primary values |
| Monospace | 13 pt | status labels, state chips, grids |
| Small | 12 pt | secondary notes, meter values |
| Scope text | 11 pt | waveform/pattern overlays |

## Spacing and rhythm

- global item spacing: `8 × 8`
- button padding: `8 × 4`
- panel margin: `10 px`
- panel title gap: `6 px`
- demo row height: `24 px`
- waveform height: `156 px`
- pattern row height: `24 px`
- pattern header height: `28 px`
- inspector meter height: `12 px`

## Focus and panel states

- focused panel border: `2 px`, `ACCENT`
- unfocused panel border: `1 px`, `BORDER`
- focused panel title: `ACCENT`
- unfocused panel title: `TEXT`
- canvases always use `PANEL_DIM_BG` with 1 px `BORDER_DIM`

## Component language

### Panels
- thick, square, appliance-like frames
- optional subtitle row for behavior hints
- explicit `FOCUSED` badge only on the active panel

### State chips
- small framed badges for render, playback, and focus state
- no iconography
- color carries urgency/status

### Read-only meters
- flat horizontal bars
- no knobs, sliders, or drag affordances
- value text always visible at the right edge

### Scopes and visualizations
- waveform, ADSR, and pattern views use simple line/cell primitives
- no blur, glow, shadows, or ant-heavy ornament
- playback cursor is a single accent line

## Non-goals

- no editing affordances
- no DAW transport strip
- no piano roll
- no glossy modern widgets
- no animated transitions beyond cursor repaint while playing
