/* ncursesw/curses.h — stub for WebAssembly build
 * The WASM build does not use ncurses; this header satisfies the
 * #include in memdeck.h so the core C modules compile with Emscripten.
 */
#ifndef CURSES_STUB_H
#define CURSES_STUB_H

/* Types and constants referenced only by ui/main (not compiled for WASM) */
typedef struct {} WINDOW;
#define COLS 80
#define LINES 24
#define COLOR_PAIR(n) (n)
#define A_BOLD 0
#define A_UNDERLINE 0
#define KEY_UP 259
#define KEY_DOWN 258

#endif /* CURSES_STUB_H */
