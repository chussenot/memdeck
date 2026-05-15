# MemDeck GUI UX Principles

This document defines the initial UX strategy for the MemDeck GUI foundation.

## Design Direction

The GUI follows a strict retro-tool direction inspired by Atari ST-era sound tools and tracker minimalism:

- Dark monochrome palette
- Strong panel/grid structure
- Keyboard-first interaction model
- Low cognitive load
- Minimal ornamentation and no glossy modern styling

## Experience Principles

### 1. Terminal-first compatibility
The GUI is an optional layer over the C engine; it must not displace terminal workflows.

### 2. One-screen clarity
The initial foundation keeps all critical actions on one screen:

- choose demo
- render
- inspect stats
- see waveform and pattern overview

No mode-heavy navigation or hidden interaction trees.

### 3. Keyboard as primary input
Required keys are first-class and always available:

- Up/Down: navigate list/actions
- Enter: execute focused action
- Space: play placeholder action
- Tab: switch focus region
- Esc: clear status or close

### 4. Progressive disclosure
Only render-relevant controls are visible. Editor/timeline concepts are intentionally absent.

### 5. Deterministic feedback
Every render action updates a clear status line and a stable stats panel to reduce ambiguity.

## Interaction Model

```mermaid
stateDiagram-v2
    [*] --> DemoBrowser
    DemoBrowser --> RenderStats: Tab
    RenderStats --> WaveformView: Tab
    WaveformView --> PatternOverview: Tab
    PatternOverview --> InstrumentInspector: Tab
    InstrumentInspector --> FxInspector: Tab
    FxInspector --> DemoBrowser: Tab

    DemoBrowser --> FxInspector: Shift+Tab
    RenderStats --> DemoBrowser: Shift+Tab
    WaveformView --> RenderStats: Shift+Tab
    PatternOverview --> WaveformView: Shift+Tab
    InstrumentInspector --> PatternOverview: Shift+Tab
    FxInspector --> InstrumentInspector: Shift+Tab

    DemoBrowser --> DemoBrowser: Up / Down (demo browse)
    RenderStats --> RenderStats: Up / Down (track select)
    WaveformView --> WaveformView: Up / Down (track select)
    PatternOverview --> PatternOverview: Up / Down (track select)
    InstrumentInspector --> InstrumentInspector: Up / Down (track select)
    FxInspector --> FxInspector: Up / Down (track select)

    DemoBrowser --> AnyPanel: D/S/W/P/I/F
    RenderStats --> AnyPanel: D/S/W/P/I/F
    WaveformView --> AnyPanel: D/S/W/P/I/F
    PatternOverview --> AnyPanel: D/S/W/P/I/F
    InstrumentInspector --> AnyPanel: D/S/W/P/I/F
    FxInspector --> AnyPanel: D/S/W/P/I/F

    DemoBrowser --> Rendered: Enter
    RenderStats --> Rendered: Enter

    Rendered --> Playing: Space
    Playing --> Rendered: Space
    Playing --> Rendered: Esc
```

## Visual System Foundations

```mermaid
flowchart TD
    A[Monochrome Base] --> B[High Contrast Text]
    A --> C[Accent Focus States]
    A --> D[Grid/Panel Rhythm]
    D --> E[Demo Browser]
    D --> F[Render Stats]
    D --> G[Waveform View]
    D --> H[Pattern Overview]
    D --> I[Instrument Inspector]
    D --> J[FX Inspector]
    D --> K[Status Line]
```

## Non-Goals (Explicit)

The GUI foundation explicitly excludes:

- DAW timeline behavior
- piano-roll editing
- tracker step editing
- sequencing/editor workflows
- audio engine rewrite in Rust
- Tauri stack adoption

## Quality Bar for the Foundation

- Fast launch
- Minimal visual noise
- Predictable key behavior
- Stable render stats visibility
- Safe memory lifecycle across C FFI boundary
