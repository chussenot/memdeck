# MemDeck GUI Keybindings

## Runtime keys

| Key | Action |
| --- | --- |
| `Up` / `Down` | Select demo |
| `Enter` | Render selected demo |
| `Space` | Play or stop the current rendered demo |
| `Tab` | Switch focus highlight between demos and overview |
| `Esc` | Stop playback |

## Interaction model

```mermaid
stateDiagram-v2
    [*] --> DemoSelection
    DemoSelection --> DemoSelection: Up / Down
    DemoSelection --> Rendered: Enter
    Rendered --> Playing: Space
    Playing --> Rendered: Space
    Playing --> Rendered: Esc
    DemoSelection --> OverviewFocus: Tab
    OverviewFocus --> DemoSelection: Tab
    OverviewFocus --> Rendered: Enter
```

## Design intent

- keyboard-first
- low cognitive load
- no mouse-heavy workflow
- no editing or DAW behavior
