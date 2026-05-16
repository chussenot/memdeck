# MemDeck GUI Keybindings

## Global/browser keys

| Key | Action |
| --- | --- |
| `Ctrl+N` | New editable song |
| `Ctrl+O` | Open editable song dialog |
| `Ctrl+D` | Duplicate demo (Browser) or duplicate editable song (Edit/Preview) |
| `Ctrl+S` | Save editable song |
| `Ctrl+Shift+S` | Save editable song as path |
| `Ctrl+R` | Render (browser demo render or editable preview render) |
| `Tab` / `Shift+Tab` | Cycle focus panels |
| `D` `S` `W` `P` `E`/`G` `I` `F` | Direct focus (demo/stats/waveform/arrangement/pattern editor/instrument/fx) |
| `Up` / `Down` | Demo select when browser focused, otherwise track select |
| `Enter` | Render selected demo (browser workflow) |
| `Space` | Start/stop playback |
| `Esc` | Cancel active dialog/rename, return focus, or stop playback |

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
| `Ctrl+S` | Save editable ABC |
| `Ctrl+Shift+S` | Save As dialog |
| `Ctrl+R` | Render editable preview through C engine |

## Inspector editing (Edit/Preview mode)

| Panel | Behavior |
| --- | --- |
| Instrument Inspector | Edit selected track instrument assignment + synthesis controls |
| FX Inspector | Edit selected track bus routing + selected bus FX values |

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
- inspector sliders/combos edit instrument/FX values in Edit/Preview mode
- no drag-and-drop in current implementation
