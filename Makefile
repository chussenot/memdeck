PREFIX  ?= /usr/local
CC      ?= cc
CFLAGS  ?= -Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600
LDFLAGS ?= -lncursesw -lm

SRC          = src/main.c src/card.c src/stack.c src/progress.c src/session.c src/ui.c src/sound.c src/abc.c src/audio_dsp.c src/audio_seq.c src/audio_mix.c src/audio_song_builtin.c src/audio_fx.c
BIN          = bin/memdeck-tui
BENCH        = bin/bench-audio
TEST_DSP     = bin/test-audio-dsp
TEST_ABC_BIN = bin/test-abc
TEST_SEQ_BIN = bin/test-audio-seq

.PHONY: all clean install uninstall test test-audio test-abc test-audio-seq bench-audio help

.DEFAULT_GOAL := help

all: $(BIN)

$(BIN): $(SRC) src/memdeck.h
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $@ $(SRC) $(LDFLAGS)
	@echo "Build complete: $(BIN)"

clean:
	rm -f $(BIN) $(BENCH) $(TEST_DSP) $(TEST_ABC_BIN) $(TEST_SEQ_BIN)

install: all
	install -d $(PREFIX)/bin
	install -m 755 bin/memdeck $(PREFIX)/bin/memdeck
	install -m 755 $(BIN) $(PREFIX)/bin/memdeck-tui
	install -d $(PREFIX)/share/memdeck/stacks
	install -d $(PREFIX)/share/memdeck/lessons
	install -d $(PREFIX)/share/memdeck/music
	install -m 644 data/stacks/*.tsv $(PREFIX)/share/memdeck/stacks/
	install -m 644 data/lessons/*.txt $(PREFIX)/share/memdeck/lessons/
	install -m 644 data/music/*.abc $(PREFIX)/share/memdeck/music/

uninstall:
	rm -f $(PREFIX)/bin/memdeck $(PREFIX)/bin/memdeck-tui
	rm -rf $(PREFIX)/share/memdeck

test: all test-audio test-abc test-audio-seq
	@echo "Running tests..."
	@sh tests/test_cards.sh
	@sh tests/test_stacks.sh
	@sh tests/test_scoring.sh
	@echo "All tests passed."

test-audio: $(TEST_DSP)
	@echo "Running audio DSP regression tests..."
	@$(TEST_DSP)

$(TEST_DSP): tests/test_audio_dsp.c src/audio_dsp.c src/audio_dsp.h
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $@ tests/test_audio_dsp.c src/audio_dsp.c $(LDFLAGS)

test-abc: $(TEST_ABC_BIN)
	@echo "Running ABC parser tests..."
	@$(TEST_ABC_BIN)

$(TEST_ABC_BIN): tests/test_abc.c src/abc.c src/card.c src/audio_dsp.c src/memdeck.h
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $@ tests/test_abc.c src/abc.c src/card.c src/audio_dsp.c $(LDFLAGS)

test-audio-seq: $(TEST_SEQ_BIN)
	@echo "Running sequencer regression tests..."
	@$(TEST_SEQ_BIN)

$(TEST_SEQ_BIN): tests/test_audio_seq.c src/audio_seq.c src/audio_mix.c src/audio_dsp.c src/audio_song_builtin.c src/audio_fx.c src/audio_seq.h src/audio_mix.h src/audio_song_builtin.h src/audio_fx.h
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $@ tests/test_audio_seq.c src/audio_seq.c src/audio_mix.c src/audio_dsp.c src/audio_song_builtin.c src/audio_fx.c $(LDFLAGS)

bench-audio: src/audio_dsp.c tests/bench_audio.c
	@mkdir -p bin
	$(CC) $(CFLAGS) -o $(BENCH) tests/bench_audio.c src/audio_dsp.c $(LDFLAGS)
	@echo "Build complete: $(BENCH)"
	@$(BENCH)

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
	@echo "  test       Run the test suite (cards, stacks, scoring, audio DSP, ABC)"
	@echo "  test-audio Build and run audio DSP regression tests"
	@echo "  test-abc   Build and run ABC parser tests"
	@echo "  test-audio-seq Build and run sequencer regression tests"
	@echo "  bench-audio Build and run audio microbenchmark"
	@echo "  help       Show this help message (default)"
