# MemDeck GUI Keybindings

## Required runtime keys

| Key | Action |
| --- | --- |
| `Up` / `Down` | Browse demos when Demo Browser is focused; otherwise change selected track |
| `Enter` | Render selected demo |
| `Space` | Start or stop playback |
| `Tab` | Cycle focus forward across all panels |
| `Shift+Tab` | Cycle focus backward across all panels |
| `Esc` | Stop playback |

## Direct panel focus

| Key | Panel |
| --- | --- |
| `D` | Demo Browser |
| `S` | Render Stats |
| `W` | Waveform |
| `P` | Pattern Overview |
| `I` | Instrument Inspector |
| `F` | FX Inspector |

## Interaction model

```mermaid
stateDiagram-v2
    [*] --> DemoBrowser
    DemoBrowser --> RenderStats: Tab
    RenderStats --> WaveformView: Tab
    WaveformView --> PatternOverview: Tab
    PatternOverview --> InstrumentInspector: Tab
    InstrumentInspector --> FxInspector: Tab
    FxInspector --> DemoBrowser: Tab

    DemoBrowser --> DemoBrowser: Up / Down
    RenderStats --> RenderStats: Up / Down (track select)
    WaveformView --> WaveformView: Up / Down (track select)
    PatternOverview --> PatternOverview: Up / Down (track select)
    InstrumentInspector --> InstrumentInspector: Up / Down (track select)
    FxInspector --> FxInspector: Up / Down (track select)

    DemoBrowser --> Rendered: Enter
    RenderStats --> Rendered: Enter
    WaveformView --> Rendered: Enter
    PatternOverview --> Rendered: Enter
    InstrumentInspector --> Rendered: Enter
    FxInspector --> Rendered: Enter

    Rendered --> Playing: Space
    Playing --> Rendered: Space
    Playing --> Rendered: Esc
```

## Intent

- keyboard-first
- low cognitive load
- explicit status reporting
- read-only transport only
