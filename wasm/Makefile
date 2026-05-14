# wasm/Makefile — build MemDeck for WebAssembly using Emscripten
#
# Prerequisites:
#   emcc (Emscripten SDK)  https://emscripten.org/docs/getting_started/
#
# Usage:
#   make          # compile to memdeck.js + memdeck.wasm in this directory
#   make clean    # remove generated files
#   make serve    # start a local HTTP server (from project root)

EMCC   ?= emcc
CFLAGS  = -O2 -std=c99 -D_DEFAULT_SOURCE -DMEMDECK_WASM \
          -I. \
          -Wno-unused-parameter -Wno-unused-function

# Core C modules — ui.c / main.c / sound.c / abc.c are excluded;
# stubs for sound_* are provided in memdeck_wasm.c.
SRC = ../src/card.c \
      ../src/stack.c \
      ../src/session.c \
      ../src/progress.c \
      memdeck_wasm.c

OUT = memdeck.js
VERIFY_OUT = verify-audio.js

EXPORTED_FUNCTIONS = \
  _md_init,\
  _md_load_stack,\
  _md_stack_count,\
  _md_stack_name,\
  _md_stack_size,\
  _md_get_current_stack,\
  _md_set_current_stack,\
  _md_get_entry,\
  _md_set_settings,\
  _md_get_settings,\
  _md_session_start,\
  _md_session_question,\
  _md_session_answer,\
  _md_session_next,\
  _md_session_is_over,\
  _md_session_stats,\
  _md_session_complete,\
  _md_progress_dump,\
  _md_progress_load,\
  _md_progress_reset,\
  _md_filtered_count,\
  _md_validate_stack

EXPORTED_RUNTIME = ccall,cwrap,UTF8ToString,stringToUTF8,lengthBytesUTF8

.PHONY: all clean serve verify

all: $(OUT)

verify: $(VERIFY_OUT)

$(OUT): $(SRC) ../src/memdeck.h ncursesw/curses.h
	$(EMCC) $(CFLAGS) $(SRC) \
	  -o $@ \
	  -s WASM=1 \
	  -s MODULARIZE=1 \
	  -s EXPORT_NAME=MemDeckModule \
	  -s EXPORTED_FUNCTIONS='[$(EXPORTED_FUNCTIONS)]' \
	  -s EXPORTED_RUNTIME_METHODS='[$(EXPORTED_RUNTIME)]' \
	  -s ALLOW_MEMORY_GROWTH=1 \
	  -s NO_EXIT_RUNTIME=1 \
	  -s ASSERTIONS=0

$(VERIFY_OUT): verify_audio.c ../src/abc.c ../src/audio_dsp.c ../src/audio_dsp.h ../src/memdeck.h
	$(EMCC) $(CFLAGS) verify_audio.c ../src/abc.c ../src/audio_dsp.c \
	  -o $@ \
	  -s WASM=1 \
	  -s ASSERTIONS=0

clean:
	rm -f memdeck.js memdeck.wasm $(VERIFY_OUT) verify-audio.wasm

# Serve from the project root so data/stacks/ is reachable if needed
serve: all
	cd .. && python3 -m http.server 8080
