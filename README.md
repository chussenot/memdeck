# MemDeck

A terminal-first trainer for memorized decks used by magicians. Practice card-at-position, position-of-card, next/previous card drills, and more — all from your terminal.

## Quick Start

```sh
# Build
make

# Run
./bin/memdeck
```

### Dependencies

- C compiler (cc/gcc/clang)
- ncurses development headers
- POSIX shell (sh)

Install ncurses headers:
```sh
# Debian/Ubuntu
sudo apt install libncurses-dev

# macOS
brew install ncurses

# Fedora
sudo dnf install ncurses-devel

# Arch
sudo pacman -S ncurses
```

## Usage

```sh
memdeck              # Launch interactive TUI
memdeck play         # Jump into practice
memdeck study        # Jump into study mode
memdeck stacks       # List available stacks
memdeck validate f   # Validate a stack file
memdeck import f     # Import a stack file
memdeck export name  # Export stack to stdout (TSV)
memdeck stats        # Show progress statistics
memdeck reset-progress  # Reset all progress
```

## Built-in Stacks

- **Aronson** — Simon Aronson's stack from *A Stack to Remember*
- **Mnemonica** — Juan Tamariz's stack from *Mnemonica*
- **Example Custom** — A simple sequential stack with sample mnemonics

## Screens

### Main Menu
Navigate between Play, Study, Stacks, Progress, Learn, and Quit.

### Practice
Multiple choice drills with immediate feedback, score tracking, and streaks.

Modes:
- Position -> Card
- Card -> Position
- Next Card / Previous Card
- Suit Drill / Value Drill
- Mixed Mode (random question types)

### Practice Settings
Configure sessions with:
- Stack selection
- Answer style (MCQ or free input)
- Number of choices (2-6)
- Card range (e.g., positions 1-26)
- Card filters (all, black, red, by suit, face cards, numbers)
- Limit mode (none, time, question count, lives)
- Mnemonic display toggle

### Study
Browse the stack one card at a time. Navigate with arrow keys, reveal mnemonics with Space, jump to any position with `g`.

### Stacks
View and manage stacks. Set the active stack, validate, or browse all entries.

### Progress
Track total sessions, accuracy, best scores, day streaks, and hardest positions.

### Learn
Read about the memorized deck method, popular stacks, and memorization techniques.

## Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate up/down |
| `h`/`l` or arrows | Navigate left/right, adjust settings |
| `Enter` | Confirm / select |
| `1`-`4` | Quick answer in MCQ |
| `Space` | Reveal mnemonic / next question |
| `g` or `/` | Jump to position (study mode) |
| `q` or `Esc` | Go back / quit |
| `?` | Help (context dependent) |

## Stack File Format

Plain TSV with optional comments:

```tsv
# Stack Name
# position	card	mnemonic
1	JS	Jack of Spades - first card
2	KC	King of Clubs - second card
...
52	10H	Ten of Hearts - last card
```

**Card codes:** `A`, `2`-`10`, `J`, `Q`, `K` followed by `S` (spades), `H` (hearts), `C` (clubs), `D` (diamonds).

Examples: `AS`, `10H`, `KD`, `3C`

### Adding a New Built-in Stack

1. Create a `.tsv` file in `data/stacks/`
2. Follow the format above with 52 unique cards at 52 unique positions
3. Validate: `memdeck validate data/stacks/your-stack.tsv`

### Custom Stacks

Import a stack file:
```sh
memdeck import my-stack.tsv
```

Custom stacks are stored in `~/.local/share/memdeck/stacks/`.

## Project Structure

```
memdeck/
  bin/memdeck          POSIX shell entrypoint
  bin/memdeck-tui      Compiled ncurses binary
  src/                 C source code
    main.c             Screens and main loop
    card.c             Card parsing and display
    stack.c            Stack loading and validation
    session.c          Practice session logic
    progress.c         Progress persistence
    ui.c               ncurses drawing utilities
    memdeck.h          Shared header
  data/
    stacks/            Built-in stack files (TSV)
    lessons/           Learning content
  tests/               Automated test scripts
  Makefile
```

## Data Storage

Progress is stored in `~/.local/share/memdeck/progress.dat` (XDG compliant). Override with `XDG_DATA_HOME`.

Stack data is stored in `data/stacks/` (built-in) and `~/.local/share/memdeck/stacks/` (custom). Override data location with `MEMDECK_DATA`.

## Testing

```sh
make test
```

## Install System-wide

```sh
sudo make install              # installs to /usr/local
sudo make PREFIX=/opt install  # custom prefix
```

## License

MIT
