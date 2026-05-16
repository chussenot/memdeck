# MemDeck GUI Keybindings

## Global/browser keys

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Cycle focus panels |
| `D` `S` `W` `P` `E`/`G` `I` `F` | Direct focus (demo/stats/waveform/arrangement/pattern editor/instrument/fx) |
| `Up` / `Down` | Demo select when browser focused, otherwise track select |
| `Enter` | Render selected demo (browser workflow) |
| `Space` | Start/stop playback |
| `Esc` | Stop playback / cancel active rename |

## Arrangement editor keys (Edit/Preview mode)

| Key | Action |
| --- | --- |
| `A` | Focus Arrangement Editor panel |
| `Left` / `Right` | Move arrangement cursor |
| `Ctrl+Left` / `Ctrl+Right` | Reorder selected block |
| `Up` / `Down` | Select track |
| `Enter` | Open selected pattern in Pattern Editor |
| `N` | Add pattern block |
| `D` | Duplicate selected block |
| `Backspace` / `Delete` | Remove selected block |
| `R` | Rename selected pattern |
| `Ctrl+S` | Save ABC |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+R` | Render preview through C engine |

## Pattern editor keys (Edit/Preview mode)

| Key | Action |
| --- | --- |
| `E` or `G` | Focus Pattern Editor panel |
| `Arrow keys` | Move step cursor |
| `Space` / `Enter` | Toggle step on/off |
| `+` / `-` | Octave up/down |
| `A` | Toggle accent |
| `F` | Toggle FX trigger |
| `G` | Edit/cycle gate |
| `V` | Edit/cycle velocity |
| `C` | Copy step |
| `X` | Cut step |
| `P` | Paste step |
| `Esc` | Return focus to Arrangement Editor |

## Mouse (minimal)

- click block: select block
- double click block: open pattern editor
- click pattern cell: select step
- double click pattern cell: toggle step
- no drag-and-drop in current implementation
