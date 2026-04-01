#include "memdeck.h"

void ui_init(void)
{
    setlocale(LC_ALL, "");
    initscr();
    cbreak();
    noecho();
    keypad(stdscr, TRUE);
    curs_set(0);
    start_color();
    use_default_colors();

    init_pair(CP_NORMAL,    -1, -1);
    init_pair(CP_TITLE,     COLOR_CYAN, -1);
    init_pair(CP_HIGHLIGHT, COLOR_BLACK, COLOR_CYAN);
    init_pair(CP_CORRECT,   COLOR_GREEN, -1);
    init_pair(CP_WRONG,     COLOR_RED, -1);
    init_pair(CP_HEARTS,    COLOR_RED, -1);
    init_pair(CP_DIAMONDS,  COLOR_RED, -1);
    init_pair(CP_SPADES,    COLOR_WHITE, -1);
    init_pair(CP_CLUBS,     COLOR_WHITE, -1);
    init_pair(CP_HELP,      COLOR_CYAN, -1);
    init_pair(CP_SCORE,     COLOR_YELLOW, -1);
    init_pair(CP_DIM,       COLOR_WHITE, -1);
    init_pair(CP_SELECTED,  COLOR_BLACK, COLOR_WHITE);
}

void ui_cleanup(void)
{
    endwin();
}

void ui_draw_box(int y, int x, int h, int w, const char *title)
{
    /* top border */
    mvaddch(y, x, ACS_ULCORNER);
    for (int i = 1; i < w - 1; i++) mvaddch(y, x + i, ACS_HLINE);
    mvaddch(y, x + w - 1, ACS_URCORNER);

    /* sides */
    for (int i = 1; i < h - 1; i++) {
        mvaddch(y + i, x, ACS_VLINE);
        mvaddch(y + i, x + w - 1, ACS_VLINE);
    }

    /* bottom border */
    mvaddch(y + h - 1, x, ACS_LLCORNER);
    for (int i = 1; i < w - 1; i++) mvaddch(y + h - 1, x + i, ACS_HLINE);
    mvaddch(y + h - 1, x + w - 1, ACS_LRCORNER);

    /* title */
    if (title && title[0]) {
        int tlen = strlen(title);
        int tx = x + (w - tlen - 2) / 2;
        if (tx < x + 1) tx = x + 1;
        mvprintw(y, tx, " %s ", title);
    }
}

void ui_draw_centered(int y, const char *text, int attr)
{
    int w = COLS;
    int len = strlen(text);
    int x = (w - len) / 2;
    if (x < 0) x = 0;
    if (attr) attron(attr);
    mvprintw(y, x, "%s", text);
    if (attr) attroff(attr);
}

void ui_draw_help_bar(const char *text)
{
    int y = LINES - 1;
    attron(COLOR_PAIR(CP_HELP));
    mvhline(y, 0, ' ', COLS);
    mvprintw(y, 1, "%s", text);
    attroff(COLOR_PAIR(CP_HELP));
}

void ui_draw_title_bar(const char *text)
{
    attron(COLOR_PAIR(CP_TITLE) | A_BOLD);
    mvhline(0, 0, ' ', COLS);
    int x = (COLS - (int)strlen(text)) / 2;
    if (x < 0) x = 0;
    mvprintw(0, x, "%s", text);
    attroff(COLOR_PAIR(CP_TITLE) | A_BOLD);
}

void ui_draw_card_fancy(int y, int x, const Card *c)
{
    char disp[16];
    card_display(c, disp, sizeof(disp));
    int cp = card_color_pair(c);
    attron(COLOR_PAIR(cp) | A_BOLD);
    mvprintw(y, x, "%s", disp);
    attroff(COLOR_PAIR(cp) | A_BOLD);
}
