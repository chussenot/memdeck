# MemDeck — WebAssembly

A browser-based reimplementation of MemDeck that compiles the core game logic
(card, stack, session, and progress modules) to **WebAssembly** via
[Emscripten](https://emscripten.org) and pairs it with a JavaScript/CSS front-end.

## Architecture

```
wasm/
  memdeck_wasm.c      C wrapper — exports game logic to JS via a JSON API
  Makefile            Emscripten build (produces memdeck.js + memdeck.wasm)
  ncursesw/curses.h   Minimal ncurses stub so memdeck.h compiles under emcc
  index.html          Single-page web app
  style.css           Dark card-magic themed CSS
  app.js              JavaScript app — all screens, Web Audio, localStorage
```

The ncurses UI (`ui.c`, `main.c`) and sound engine (`sound.c`, `abc.c`) are
**not** compiled for WASM.  Sound is handled by the Web Audio API in `app.js`,
and progress is persisted to `localStorage` instead of a flat file.

## Build

### Prerequisites

```sh
# Install Emscripten SDK
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk
./emsdk install latest
./emsdk activate latest
source ./emsdk_env.sh   # or emsdk_env.bat on Windows
```

### Compile

```sh
cd wasm
make          # produces memdeck.js and memdeck.wasm in wasm/
make clean    # remove generated artefacts
```

### Run

```sh
# From the wasm/ directory — starts Python HTTP server on port 8080
make serve

# Then open http://localhost:8080/wasm/
```

> **Note:** The app must be served over HTTP (not `file://`) because browsers
> block WASM loading from local file paths.

## Features

All practice modes from the TUI are available:

| Mode             | Description                                 |
|------------------|---------------------------------------------|
| Position → Card  | Given a position, name the card             |
| Card → Position  | Given a card, name its position             |
| Next Card        | Name the card that follows a given card     |
| Previous Card    | Name the card that precedes a given card    |
| Suit Drill       | Name only the suit at a given position      |
| Value Drill      | Name only the value at a given position     |
| Mixed            | Random mix of all question types            |

- **3 built-in stacks** — Aronson, Mnemonica, Memorandum (embedded in JS)
- **Configurable settings** — choices (2–6), card range, filter, limits
- **Progress tracking** — per-session and per-card stats, persisted to `localStorage`
- **Study mode** — browse the stack card by card with mnemonic reveal
- **Chiptune sounds** — Web Audio API square-wave feedback (success/fail)
- **Learn screen** — introduction to memorized deck technique
