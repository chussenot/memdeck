# MemDeck GUI Layout

The first stable runtime screen keeps the interface compact, deterministic, and Atari/ST-inspired.

## Primary screen

```text
+------------------------------------------------+
| MEMDECK SOUND MACHINE                          |
+----------------------+-------------------------+
| DEMOS                | RENDER STATS            |
| > dark_moroder       | duration                |
|   neon_nightdrive    | checksum                |
|   metro_chase        | clipping                |
|   black_sunrise      | peak                    |
+----------------------+-------------------------+
| WAVEFORM / PATTERN OVERVIEW                    |
+------------------------------------------------+
| [Tab] Focus  [Enter] Render  [Space] Play      |
| [Esc] Stop                                     |
+------------------------------------------------+
```

## Panel roles

### Demos
- fixed showcase demo list
- keyboard-first selection
- no editor controls
- no transport complexity

### Render stats
- current demo metadata
- deterministic render metrics
- playback state
- render success/failure visibility

### Waveform / pattern overview
- rendered PCM minimap
- clipping markers
- amplitude envelope lines
- arrangement blocks
- step activity rows
- track + instrument labels

## Visual system rules

- dark monochrome base
- single green accent for focus and activity
- red warning accent for clipping/errors
- strong rectangular borders
- monospace typography only
- no gradients or animation
