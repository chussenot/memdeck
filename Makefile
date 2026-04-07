PREFIX  ?= /usr/local
CC      ?= cc
CFLAGS  ?= -Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600
LDFLAGS ?= -lncursesw -lm

SRC     = src/main.c src/card.c src/stack.c src/progress.c src/session.c src/ui.c src/sound.c
BIN     = bin/memdeck-tui

.PHONY: all clean install uninstall test help

.DEFAULT_GOAL := help

all: $(BIN)

$(BIN): $(SRC) src/memdeck.h
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $@ $(SRC) $(LDFLAGS)
	@echo "Build complete: $(BIN)"

clean:
	rm -f $(BIN)

install: all
	install -d $(PREFIX)/bin
	install -m 755 bin/memdeck $(PREFIX)/bin/memdeck
	install -m 755 $(BIN) $(PREFIX)/bin/memdeck-tui
	install -d $(PREFIX)/share/memdeck/stacks
	install -d $(PREFIX)/share/memdeck/lessons
	install -m 644 data/stacks/*.tsv $(PREFIX)/share/memdeck/stacks/
	install -m 644 data/lessons/*.txt $(PREFIX)/share/memdeck/lessons/

uninstall:
	rm -f $(PREFIX)/bin/memdeck $(PREFIX)/bin/memdeck-tui
	rm -rf $(PREFIX)/share/memdeck

test: all
	@echo "Running tests..."
	@sh tests/test_cards.sh
	@sh tests/test_stacks.sh
	@sh tests/test_scoring.sh
	@echo "All tests passed."

help:
	@echo "MemDeck - Memorized Deck Trainer"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  all        Build the memdeck-tui binary"
	@echo "  clean      Remove compiled binary"
	@echo "  install    Install to $(PREFIX) (may need sudo)"
	@echo "  uninstall  Remove installed files from $(PREFIX)"
	@echo "  test       Run the test suite"
	@echo "  help       Show this help message (default)"
