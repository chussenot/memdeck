# MemDeck GUI Song Workflow

This document defines the stable song/project lifecycle for the Rust GUI editor.

## Song sources

- **Bundled demos**: read-only showcase files from `data/music/*.abc`.
- **Editable user songs**: saved to `~/.local/share/memdeck/music/user/` by default (`XDG_DATA_HOME` and `MEMDECK_USER_SONG_DIR` are supported).

Bundled demo files are never overwritten by normal **Save**. Use **Save As** for explicit path overrides.

## Lifecycle actions

Top bar and shortcuts expose:

- **New Song** (`Ctrl+N`)
- **Open Song** (`Ctrl+O`)
- **Duplicate Demo as Editable** (`Ctrl+D` from Browser mode)
- **Duplicate current editable song** (`Ctrl+D` from Edit/Preview mode)
- **Save** (`Ctrl+S`)
- **Save As** (`Ctrl+Shift+S`)
- **Close Song** (returns to Browser mode)
- **Browser Mode** (non-destructive mode switch)
- **Preview render** (`Ctrl+R`)

## Dirty-state behavior

Dirty state is tracked for:

- arrangement edits
- pattern edits
- instrument edits
- FX edits
- metadata edits (tempo/swing/pattern naming/song edits)

Dirty visibility:

- status line (`DIRTY YES/NO`)
- window title (`*` prefix when dirty in Edit/Preview)
- arrangement footer (`DIRTY YES/NO`)

## Unsaved-changes protection

When dirty, the editor blocks destructive navigation and opens a minimal modal:

- **Save**
- **Discard**
- **Cancel** (`Esc`)

Guarded transitions include:

- opening another song
- creating a new song over an unsaved session
- duplicating another song/demo
- closing the current song
- quitting the application

## Save/load ergonomics

- **Save** writes to the current source path when valid, otherwise suggests a user-song path.
- **Save As** always uses explicit path entry.
- **Open Song** uses a keyboard-first path dialog with recent entries.
- errors (invalid path, parse failure, serialization failure, IO failure) are surfaced in status + `LAST ERROR`.
- save/load uses deterministic ABC serialization and validated parser roundtrip.

## Recent songs

The browser panel and open dialog show a compact recent list with source labels:

- opened editable songs
- saved editable songs
- duplicated demo/song entries

This is intentionally lightweight (no project database/workspace layer).
