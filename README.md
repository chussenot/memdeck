# MemDeck

[![asciicast](https://asciinema.org/a/nUGcYvN3fqNC9JFi.svg)](https://asciinema.org/a/nUGcYvN3fqNC9JFi)

A terminal-first trainer for memorized decks used by card magicians. Practice card-at-position, position-of-card, next/previous card drills, suit and value drills — all from your terminal with chiptune sounds, rainbow animations, and mouse support.

## Rust GUI Status

MemDeck also includes a Rust GUI in `gui/` with preserved browser mode plus a lightweight editable workflow.

- Atari/ST-inspired composition workstation presentation
- browser/edit/preview modes
- demo browser, render stats, waveform view, arrangement editor, pattern editor, instrument inspector, FX inspector
- instrument and FX inspectors are editable in Edit/Preview mode
- keyboard-first workflow
- editable flow stays on the existing C engine path (`EditableSong -> ABC DSL -> C render -> PCM`)
- no piano roll, no full DAW behavior

### Run the GUI

```sh
make gui-check
make gui-test
make gui-run
```

On Linux, the GUI audio backend needs ALSA development headers:

```sh
sudo apt install libasound2-dev
```

On headless Linux environments:

```sh
xvfb-run -a cargo run --manifest-path gui/Cargo.toml
```

### GUI Keybindings

| Key | Action |
| --- | --- |
| `Up` / `Down` | Browse demos when Demo Browser is focused; otherwise change selected track |
| `Enter` | Render selected demo |
| `Space` | Start or stop playback |
| `Esc` | Cancel dialog/rename, return focus, or stop playback |
| `Tab` | Cycle focus across GUI panels |
| `D` / `S` / `W` / `P` / `E`/`G` / `I` / `F` | Focus Demo / Stats / Waveform / Arrangement / Pattern Editor / Instrument / FX panels |
| `Ctrl+N` / `Ctrl+O` | New song / Open song dialog |
| `Ctrl+D` | Duplicate demo (Browser) or duplicate editable song (Edit/Preview) |
| `Ctrl+S` / `Ctrl+Shift+S` | Save editable song / Save As |
| `Ctrl+R` | Render selected demo (Browser) or editable preview (Edit/Preview) |

### GUI Limitations

- GUI playback uses an in-process Rust audio stream backend (`cpal`)
- editable step metadata is intentionally minimal and keyboard-driven
- arrangement integration is block-based only (no advanced timeline editing)
- the GUI remains a shell over the existing C renderer, not a new DAW engine
- screenshots and architecture notes live under `docs/gui-*.md` (capture workflow: `docs/gui-screenshot-guide.md`)
- editable songs default to `~/.local/share/memdeck/music/user/` (`XDG_DATA_HOME` and `MEMDECK_USER_SONG_DIR` supported)
- full song lifecycle details: `docs/gui-song-workflow.md`

## Quick Start

```sh
make all     # Build
./bin/memdeck  # Run
```

### Dependencies

- C compiler (cc/gcc/clang)
- ncurses wide-character headers (`libncursesw`)
- `aplay` (ALSA utils) for chiptune audio
- POSIX shell (sh)

```sh
# Debian/Ubuntu
sudo apt install libncursesw5-dev alsa-utils

# macOS
brew install ncurses

# Fedora
sudo dnf install ncurses-devel alsa-utils

# Arch
sudo pacman -S ncurses alsa-utils
```

## Features

- **7 practice modes** — position-to-card, card-to-position, next/previous card, suit drill, value drill, and mixed mode
- **Configurable sessions** — 2-6 MCQ choices, card range, suit/color/face filters, time/question/lives limits
- **Chiptune sound engine** — reusable song/pattern sequencer for dark synth / retro electro loops plus success/fail sound effects
- **Animated rainbow logo** — mirage heat-shimmer effect with fast color cycling
- **Mouse support** — click to navigate all menus and answer questions
- **Progress tracking** — persistent stats, day streaks, per-card error tracking, hardest positions
- **3 built-in stacks** — Aronson, Mnemonica, and Memorandum
- **Custom stacks** — import your own TSV files
- **ASCII card art** — colored suit symbols with proper card layout

## Usage

### Interactive

```sh
memdeck           # Launch the TUI
memdeck play      # Jump straight into practice
memdeck study     # Jump into study mode
```

### CLI Commands

```sh
memdeck stacks           # List available stacks
memdeck validate FILE    # Validate a stack TSV file
memdeck import FILE      # Import a stack to your collection
memdeck export NAME      # Export a stack to stdout (TSV)
memdeck stats            # Show progress statistics
memdeck reset-progress   # Reset all progress data
```

## Screens

| Screen | Description |
|--------|-------------|
| **Menu** | Animated home screen with rainbow logo and background music |
| **Play** | Choose a practice mode (7 drill types) |
| **Practice** | Answer MCQ questions with immediate feedback and sound effects |
| **Settings** | Configure stack, choices, range, filters, limits, mnemonics |
| **Study** | Browse the stack card by card, reveal mnemonics |
| **Stacks** | View, validate, and set active stack |
| **Progress** | Stats, accuracy, streaks, hardest positions, reset button |
| **Learn** | Read about the memorized deck method |

## Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate up/down |
| `h`/`l` or arrows | Adjust settings left/right |
| `Enter` or click | Select / confirm |
| `1`-`6` | Quick answer in practice |
| `Space` | Reveal mnemonic / next question |
| `g` / `G` | Jump to start / end |
| `r` | Reset progress (on progress screen) |
| `q` / `Esc` | Go back / quit |
| Mouse click | Navigate menus, select answers |

## Stack File Format

Plain TSV with optional comments and mnemonics:

```tsv
# My Custom Stack
# position  card  mnemonic
1	JS	Jack holds a Spade
2	KC	King wears a Crown
...
52	10H	Ten red Hearts
```

Card codes: `A`, `2`-`10`, `J`, `Q`, `K` followed by `S` (spades), `H` (hearts), `C` (clubs), `D` (diamonds). Examples: `AS`, `10H`, `KD`, `3C`.

Custom stacks are stored in `~/.local/share/memdeck/stacks/`.

## Build Targets

```
make          # Show help (default)
make all      # Build the binary
make test     # Run test suite
make gui-check # Check the Rust GUI crate
make gui-test  # Test the Rust GUI crate
make gui-run   # Launch the Rust GUI crate
make render-demos  # Render all showcase tracks with deterministic metrics
make play-demo DEMO=dark_moroder  # Render one demo quickly
make test-audio-seq  # Run sequencer regression tests
make install  # Install to /usr/local (sudo)
make clean    # Remove binary
```

## Project Structure

```
memdeck/
  bin/
    memdeck          POSIX shell wrapper (CLI commands)
    memdeck-tui      Compiled ncurses binary
  src/
    main.c           Screens, animation, main loop
    session.c        Practice session and answer checking
    card.c           Card parsing and display
    stack.c          Stack loading and validation
    progress.c       Progress persistence
    ui.c             ncurses drawing, card art, colors
    sound.c          Native audio backend and SFX playback
    audio_seq.c      Song / pattern / track / step sequencer
    audio_mix.c      Portable PCM mixing pipeline
    audio_fx.c       Sequencer FX (drive/lowpass/delay/sidechain) processing
    audio_engine.c   Audio render API (built-in and ABC file rendering)
    audio_song_builtin.c Built-in fallback retro song data
    audio_dsp.c      Portable oscillator and timing core
    memdeck.h        Shared types and constants
  data/
    stacks/          Built-in stack files (TSV)
    lessons/         Learning content
    music/           ABC tracks and showcase demos
      dark_moroder.abc
      perturbator_loop.abc
      carpenter_drive.abc
      advanced_dsl_demo.abc
      multi_fx_demo.abc
      neon_nightdrive.abc
      metro_chase.abc
      black_sunrise.abc
      machine_romance.abc
      hypersleep_dream.abc
  docs/
    composer-guide.md  Composition workflow and preset guidance
    showcase-tracks.md Showcase track notes and catalog
    gui-*.md          Rust GUI architecture, layout, runtime, and design docs
  tests/             Shell-based test scripts
  Makefile
```

## Data Storage

Progress: `~/.local/share/memdeck/progress.dat` (XDG compliant, override with `XDG_DATA_HOME`)

Custom stacks: `~/.local/share/memdeck/stacks/` (override data location with `MEMDECK_DATA`)

## License

MIT
