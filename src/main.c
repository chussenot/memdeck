#include "memdeck.h"

/* ─── Main Menu ──────────────────────────────────────────────── */

static const char *menu_items[] = {
    "Play", "Study", "Stacks", "Progress", "Learn", "Quit"
};
#define MENU_COUNT 6

int screen_menu(App *app)
{
    int ch;
    for (;;) {
        erase();
        ui_draw_title_bar("MEMDECK");

        /* logo */
        int cy = LINES / 2 - MENU_COUNT - 2;
        if (cy < 3) cy = 3;

        attron(COLOR_PAIR(CP_TITLE) | A_BOLD);
        ui_draw_centered(cy,     "  __  __                ____            _    ", 0);
        ui_draw_centered(cy + 1, " |  \\/  | ___ _ __ ___|  _ \\  ___  ___| | __", 0);
        ui_draw_centered(cy + 2, " | |\\/| |/ _ \\ '_ ` _ \\ | | |/ _ \\/ __| |/ /", 0);
        ui_draw_centered(cy + 3, " | |  | |  __/ | | | | | |_| |  __/ (__|   < ", 0);
        ui_draw_centered(cy + 4, " |_|  |_|\\___|_| |_| |_|____/ \\___|\\___|_|\\_\\", 0);
        attroff(COLOR_PAIR(CP_TITLE) | A_BOLD);

        ui_draw_centered(cy + 6, "Memorized Deck Trainer", COLOR_PAIR(CP_DIM));

        /* stack info */
        if (app->stack_count > 0) {
            char info[128];
            snprintf(info, sizeof(info), "Current stack: %s (%d cards)",
                     app->stacks[app->current_stack].name,
                     app->stacks[app->current_stack].count);
            ui_draw_centered(cy + 8, info, COLOR_PAIR(CP_DIM));
        }

        /* menu items */
        int my = cy + 10;
        for (int i = 0; i < MENU_COUNT; i++) {
            if (i == app->menu_sel) {
                attron(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
                mvprintw(my + i * 2, COLS / 2 - 10, "  > %-20s", menu_items[i]);
                attroff(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
            } else {
                mvprintw(my + i * 2, COLS / 2 - 10, "    %-20s", menu_items[i]);
            }
        }

        /* streak info */
        if (app->progress.current_streak > 0) {
            char streak[64];
            snprintf(streak, sizeof(streak), "Day streak: %d", app->progress.current_streak);
            ui_draw_centered(LINES - 3, streak, COLOR_PAIR(CP_SCORE) | A_BOLD);
        }

        ui_draw_help_bar("j/k or arrows: navigate  Enter: select  q: quit");
        refresh();

        ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            app->menu_sel = (app->menu_sel - 1 + MENU_COUNT) % MENU_COUNT;
            break;
        case 'j': case KEY_DOWN:
            app->menu_sel = (app->menu_sel + 1) % MENU_COUNT;
            break;
        case '\n': case KEY_ENTER:
            switch (app->menu_sel) {
            case 0: return SCREEN_PLAY;
            case 1: return SCREEN_STUDY;
            case 2: return SCREEN_STACKS;
            case 3: return SCREEN_PROGRESS;
            case 4: return SCREEN_LEARN;
            case 5: return SCREEN_QUIT;
            }
            break;
        case 'q':
            return SCREEN_QUIT;
        case 'p': case '1':
            return SCREEN_PLAY;
        case 's': case '2':
            return SCREEN_STUDY;
        case '3':
            return SCREEN_STACKS;
        case '4':
            return SCREEN_PROGRESS;
        case 'l': case '5':
            return SCREEN_LEARN;
        }
    }
}

/* ─── Play Menu ──────────────────────────────────────────────── */

static const char *play_items[] = {
    "Position -> Card",
    "Card -> Position",
    "Next Card",
    "Previous Card",
    "Suit Drill",
    "Value Drill",
    "Mixed Mode",
    "Practice Settings",
    "Back"
};
#define PLAY_COUNT 9

int screen_play_menu(App *app)
{
    int ch;
    for (;;) {
        erase();
        ui_draw_title_bar("PRACTICE MODE");

        int cy = 4;
        ui_draw_centered(cy, "Choose your practice mode:", COLOR_PAIR(CP_DIM));

        int my = cy + 3;
        for (int i = 0; i < PLAY_COUNT; i++) {
            if (i == app->play_sel) {
                attron(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
                mvprintw(my + i * 2, COLS / 2 - 15, "  > %-30s", play_items[i]);
                attroff(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
            } else {
                mvprintw(my + i * 2, COLS / 2 - 15, "    %-30s", play_items[i]);
            }
        }

        char stack_info[128];
        snprintf(stack_info, sizeof(stack_info), "Stack: %s | Range: %d-%d | %s",
                 app->stacks[app->current_stack].name,
                 app->settings.range_min, app->settings.range_max,
                 app->settings.answer_style == ANSWER_MCQ ? "Multiple Choice" : "Free Input");
        ui_draw_centered(LINES - 3, stack_info, COLOR_PAIR(CP_DIM));
        ui_draw_help_bar("j/k: navigate  Enter: select  s: settings  Esc: back");
        refresh();

        ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            app->play_sel = (app->play_sel - 1 + PLAY_COUNT) % PLAY_COUNT;
            break;
        case 'j': case KEY_DOWN:
            app->play_sel = (app->play_sel + 1) % PLAY_COUNT;
            break;
        case '\n': case KEY_ENTER:
            if (app->play_sel == PLAY_COUNT - 1) return SCREEN_MENU;
            if (app->play_sel == PLAY_COUNT - 2) return SCREEN_SETTINGS;
            app->settings.question_type = app->play_sel;
            return SCREEN_PRACTICE;
        case 's':
            return SCREEN_SETTINGS;
        case 27: case 'q':
            return SCREEN_MENU;
        }
    }
}

/* ─── Practice Screen ────────────────────────────────────────── */

int screen_practice(App *app)
{
    if (app->stack_count == 0) return SCREEN_MENU;

    session_init(app);
    Session *ss = &app->session;
    Stack *st = &app->stacks[app->current_stack];

    if (!ss->active || ss->filtered_count == 0) {
        erase();
        ui_draw_centered(LINES / 2, "No cards match your filter settings!", COLOR_PAIR(CP_WRONG));
        ui_draw_help_bar("Press any key to go back");
        refresh();
        getch();
        return SCREEN_PLAY;
    }

    int nc = app->settings.num_choices;
    if (nc > ss->filtered_count) nc = ss->filtered_count;

    for (;;) {
        erase();

        /* title bar with score */
        attron(COLOR_PAIR(CP_TITLE) | A_BOLD);
        mvhline(0, 0, ' ', COLS);
        mvprintw(0, 1, "PRACTICE");
        attroff(COLOR_PAIR(CP_TITLE) | A_BOLD);

        /* score bar */
        char score_str[128];
        if (app->settings.limit_mode == LIMIT_LIVES) {
            char hearts[32] = {0};
            for (int i = 0; i < ss->lives && i < 10; i++)
                strcat(hearts, "\xe2\x99\xa5 ");
            snprintf(score_str, sizeof(score_str),
                     "%d/%d correct  |  Streak: %d  |  %s",
                     ss->correct, ss->questions_asked, ss->streak, hearts);
        } else if (app->settings.limit_mode == LIMIT_QUESTIONS) {
            snprintf(score_str, sizeof(score_str),
                     "%d/%d correct  |  Streak: %d  |  Q %d/%d",
                     ss->correct, ss->questions_asked, ss->streak,
                     ss->questions_asked + (ss->answered ? 0 : 1),
                     app->settings.limit_value);
        } else if (app->settings.limit_mode == LIMIT_TIME) {
            int elapsed = (int)(time(NULL) - ss->start_time);
            int remaining = app->settings.limit_value - elapsed;
            if (remaining < 0) remaining = 0;
            snprintf(score_str, sizeof(score_str),
                     "%d/%d correct  |  Streak: %d  |  %d:%02d left",
                     ss->correct, ss->questions_asked, ss->streak,
                     remaining / 60, remaining % 60);
        } else {
            snprintf(score_str, sizeof(score_str),
                     "%d/%d correct  |  Streak: %d  |  Best: %d",
                     ss->correct, ss->questions_asked, ss->streak, ss->best_streak);
        }
        attron(COLOR_PAIR(CP_SCORE));
        ui_draw_centered(2, score_str, 0);
        attroff(COLOR_PAIR(CP_SCORE));

        /* question */
        int qy = LINES / 2 - nc - 1;
        if (qy < 5) qy = 5;

        attron(A_BOLD);
        ui_draw_centered(qy, ss->question_text, 0);
        attroff(A_BOLD);

        /* choices */
        int cy = qy + 3;
        for (int i = 0; i < nc; i++) {
            int idx = ss->choices[i];
            StackEntry *ce = &st->entries[idx];

            char label[128];
            char card_disp[16];
            card_display(&ce->card, card_disp, sizeof(card_disp));

            /* format choice based on question type */
            if (strstr(ss->question_text, "position") && !strstr(ss->question_text, "at position")) {
                snprintf(label, sizeof(label), "  %d) Position %d", i + 1, ce->position);
            } else if (strstr(ss->question_text, "suit")) {
                snprintf(label, sizeof(label), "  %d) %s %s", i + 1,
                         card_suit_symbol(ce->card.suit), card_suit_name(ce->card.suit));
            } else if (strstr(ss->question_text, "value")) {
                snprintf(label, sizeof(label), "  %d) %s", i + 1, card_rank_name(ce->card.rank));
            } else {
                snprintf(label, sizeof(label), "  %d) %s  (%s of %s)", i + 1, card_disp,
                         card_rank_name(ce->card.rank), card_suit_name(ce->card.suit));
            }

            int is_correct_choice = (idx == ss->current_answer);
            int is_selected = (ss->answered && i == ss->selected);

            if (ss->answered) {
                if (is_correct_choice) {
                    attron(COLOR_PAIR(CP_CORRECT) | A_BOLD);
                    mvprintw(cy + i * 2, COLS / 2 - 20, "%-40s", label);
                    if (is_selected) printw(" <-- CORRECT!");
                    else printw(" <-- answer");
                    attroff(COLOR_PAIR(CP_CORRECT) | A_BOLD);
                } else if (is_selected) {
                    attron(COLOR_PAIR(CP_WRONG) | A_BOLD);
                    mvprintw(cy + i * 2, COLS / 2 - 20, "%-40s", label);
                    printw(" <-- wrong");
                    attroff(COLOR_PAIR(CP_WRONG) | A_BOLD);
                } else {
                    attron(COLOR_PAIR(CP_DIM));
                    mvprintw(cy + i * 2, COLS / 2 - 20, "%-40s", label);
                    attroff(COLOR_PAIR(CP_DIM));
                }
            } else {
                if (i == ss->selected) {
                    attron(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
                } else {
                    attron(COLOR_PAIR(card_color_pair(&ce->card)));
                }
                mvprintw(cy + i * 2, COLS / 2 - 20, "%-40s", label);
                if (i == ss->selected)
                    attroff(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
                else
                    attroff(COLOR_PAIR(card_color_pair(&ce->card)));
            }
        }

        /* mnemonic display after answer */
        if (ss->answered && app->settings.show_mnemonic) {
            StackEntry *ae = &st->entries[ss->current_answer];
            if (ae->mnemonic[0]) {
                char mn[300];
                snprintf(mn, sizeof(mn), "Mnemonic: %s", ae->mnemonic);
                ui_draw_centered(cy + nc * 2 + 1, mn, COLOR_PAIR(CP_DIM));
            }
        }

        if (ss->answered) {
            ui_draw_help_bar("Enter/Space: next question  q: end session");
        } else {
            ui_draw_help_bar("1-4: answer  j/k: select  Enter: confirm  q: quit");
        }
        refresh();

        /* handle time limit */
        if (app->settings.limit_mode == LIMIT_TIME) {
            timeout(1000);
        }

        int ch = getch();
        timeout(-1);

        if (ch == ERR && app->settings.limit_mode == LIMIT_TIME) {
            if (session_is_over(app)) return SCREEN_COMPLETE;
            continue;
        }

        if (ch == 'q' || ch == 27) {
            ss->active = 0;
            return SCREEN_COMPLETE;
        }

        if (!ss->answered) {
            switch (ch) {
            case 'k': case KEY_UP:
                ss->selected = (ss->selected - 1 + nc) % nc;
                break;
            case 'j': case KEY_DOWN:
                ss->selected = (ss->selected + 1) % nc;
                break;
            case '1': case '2': case '3': case '4': case '5': case '6': {
                int idx = ch - '1';
                if (idx < nc) {
                    ss->selected = idx;
                    session_check_answer(app, idx);
                }
                break;
            }
            case '\n': case KEY_ENTER: case ' ':
                session_check_answer(app, ss->selected);
                break;
            }
        } else {
            if (ch == '\n' || ch == KEY_ENTER || ch == ' ') {
                if (session_is_over(app)) return SCREEN_COMPLETE;
                session_generate_question(app);
            }
        }
    }
}

/* ─── Practice Complete ──────────────────────────────────────── */

int screen_complete(App *app)
{
    Session *ss = &app->session;

    /* update progress */
    app->progress.total_sessions++;
    int score = (ss->questions_asked > 0)
        ? (ss->correct * 100 / ss->questions_asked) : 0;
    if (score > app->progress.best_score)
        app->progress.best_score = score;

    /* update daily streak */
    const char *today = progress_today();
    if (strcmp(app->progress.last_date, today) != 0) {
        /* check if yesterday */
        /* simple: just increment if last_date is set */
        if (app->progress.last_date[0])
            app->progress.current_streak++;
        else
            app->progress.current_streak = 1;
        strncpy(app->progress.last_date, today, 15);
    }
    if (app->progress.current_streak > app->progress.best_streak)
        app->progress.best_streak = app->progress.current_streak;

    progress_save(app);

    for (;;) {
        erase();
        ui_draw_title_bar("SESSION COMPLETE");

        int cy = LINES / 2 - 8;
        if (cy < 4) cy = 4;

        /* big score */
        char score_text[32];
        snprintf(score_text, sizeof(score_text), "%d%%", score);
        attron(COLOR_PAIR(score >= 80 ? CP_CORRECT : (score >= 50 ? CP_SCORE : CP_WRONG)) | A_BOLD);
        ui_draw_centered(cy, score_text, 0);
        attroff(COLOR_PAIR(score >= 80 ? CP_CORRECT : (score >= 50 ? CP_SCORE : CP_WRONG)) | A_BOLD);

        char detail[128];
        snprintf(detail, sizeof(detail), "%d correct out of %d questions",
                 ss->correct, ss->questions_asked);
        ui_draw_centered(cy + 2, detail, COLOR_PAIR(CP_DIM));

        snprintf(detail, sizeof(detail), "Best streak this session: %d", ss->best_streak);
        ui_draw_centered(cy + 4, detail, 0);

        /* separator */
        mvhline(cy + 6, COLS / 4, ACS_HLINE, COLS / 2);

        snprintf(detail, sizeof(detail), "Day streak: %d days", app->progress.current_streak);
        attron(COLOR_PAIR(CP_SCORE) | A_BOLD);
        ui_draw_centered(cy + 8, detail, 0);
        attroff(COLOR_PAIR(CP_SCORE) | A_BOLD);

        snprintf(detail, sizeof(detail), "All-time best: %d%%", app->progress.best_score);
        ui_draw_centered(cy + 10, detail, COLOR_PAIR(CP_DIM));

        /* encouragement */
        const char *msg;
        if (score == 100) msg = "Perfect! You're a stack master!";
        else if (score >= 90) msg = "Excellent work! Almost perfect!";
        else if (score >= 75) msg = "Great job! Keep practicing!";
        else if (score >= 50) msg = "Good effort! Practice makes perfect.";
        else msg = "Keep at it! Review the stack in Study mode.";

        attron(A_BOLD);
        ui_draw_centered(cy + 13, msg, 0);
        attroff(A_BOLD);

        ui_draw_help_bar("Enter: play again  m: main menu  q: quit");
        refresh();

        int ch = getch();
        switch (ch) {
        case '\n': case KEY_ENTER: case ' ':
            return SCREEN_PRACTICE;
        case 'm':
            return SCREEN_MENU;
        case 'q': case 27:
            return SCREEN_MENU;
        }
    }
}

/* ─── Practice Settings ──────────────────────────────────────── */

static const char *filter_names[] = {
    "All Cards", "Black Cards", "Red Cards",
    "Hearts Only", "Spades Only", "Clubs Only", "Diamonds Only",
    "Face Cards", "Number Cards"
};
#define FILTER_COUNT 9

static const char *limit_names[] = { "None", "Time", "Questions", "Lives" };
#define LIMIT_COUNT 4

#define SETTINGS_ITEMS 10

int screen_settings(App *app)
{
    PracticeSettings *ps = &app->settings;
    int sel = app->settings_sel;

    for (;;) {
        erase();
        ui_draw_title_bar("PRACTICE SETTINGS");

        int y = 4;
        int lx = COLS / 2 - 20;
        int vx = COLS / 2 + 5;

        for (int i = 0; i < SETTINGS_ITEMS; i++) {
            int attr = (i == sel) ? (COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD) : 0;

            if (attr) attron(attr);

            switch (i) {
            case 0:
                mvprintw(y, lx, "Stack:");
                mvprintw(y, vx, "< %s >", app->stacks[app->current_stack].name);
                break;
            case 1:
                mvprintw(y, lx, "Answer Style:");
                mvprintw(y, vx, "< %s >",
                         ps->answer_style == ANSWER_MCQ ? "Multiple Choice" : "Free Input");
                break;
            case 2:
                mvprintw(y, lx, "Choices:");
                mvprintw(y, vx, "< %d >", ps->num_choices);
                break;
            case 3:
                mvprintw(y, lx, "Range Min:");
                mvprintw(y, vx, "< %d >", ps->range_min);
                break;
            case 4:
                mvprintw(y, lx, "Range Max:");
                mvprintw(y, vx, "< %d >", ps->range_max);
                break;
            case 5:
                mvprintw(y, lx, "Card Filter:");
                mvprintw(y, vx, "< %s >", filter_names[ps->card_filter]);
                break;
            case 6:
                mvprintw(y, lx, "Limit Mode:");
                mvprintw(y, vx, "< %s >", limit_names[ps->limit_mode]);
                break;
            case 7:
                mvprintw(y, lx, "Limit Value:");
                if (ps->limit_mode == LIMIT_TIME)
                    mvprintw(y, vx, "< %d seconds >", ps->limit_value);
                else if (ps->limit_mode == LIMIT_QUESTIONS)
                    mvprintw(y, vx, "< %d questions >", ps->limit_value);
                else if (ps->limit_mode == LIMIT_LIVES)
                    mvprintw(y, vx, "< %d lives >", ps->limit_value);
                else
                    mvprintw(y, vx, "  n/a");
                break;
            case 8:
                mvprintw(y, lx, "Show Mnemonic:");
                mvprintw(y, vx, "< %s >", ps->show_mnemonic ? "Yes" : "No");
                break;
            case 9:
                mvprintw(y, lx, "Randomize:");
                mvprintw(y, vx, "< %s >", ps->randomize ? "Yes" : "No");
                break;
            }

            if (attr) attroff(attr);
            y += 2;
        }

        ui_draw_help_bar("j/k: navigate  h/l or arrows: adjust  Enter: done  Esc: cancel");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            sel = (sel - 1 + SETTINGS_ITEMS) % SETTINGS_ITEMS;
            break;
        case 'j': case KEY_DOWN:
            sel = (sel + 1) % SETTINGS_ITEMS;
            break;
        case 'l': case KEY_RIGHT: /* increase */
            switch (sel) {
            case 0:
                app->current_stack = (app->current_stack + 1) % app->stack_count;
                break;
            case 1:
                ps->answer_style = (ps->answer_style + 1) % 2;
                break;
            case 2:
                if (ps->num_choices < MAX_CHOICES) ps->num_choices++;
                break;
            case 3:
                if (ps->range_min < ps->range_max) ps->range_min++;
                break;
            case 4:
                if (ps->range_max < 52) ps->range_max++;
                break;
            case 5:
                ps->card_filter = (ps->card_filter + 1) % FILTER_COUNT;
                break;
            case 6:
                ps->limit_mode = (ps->limit_mode + 1) % LIMIT_COUNT;
                if (ps->limit_mode == LIMIT_TIME && ps->limit_value < 30) ps->limit_value = 60;
                if (ps->limit_mode == LIMIT_QUESTIONS && ps->limit_value < 5) ps->limit_value = 20;
                if (ps->limit_mode == LIMIT_LIVES && ps->limit_value < 1) ps->limit_value = 3;
                break;
            case 7:
                if (ps->limit_mode == LIMIT_TIME) ps->limit_value += 10;
                else if (ps->limit_mode != LIMIT_NONE) ps->limit_value++;
                break;
            case 8:
                ps->show_mnemonic = !ps->show_mnemonic;
                break;
            case 9:
                ps->randomize = !ps->randomize;
                break;
            }
            break;
        case 'h': case KEY_LEFT: /* decrease */
            switch (sel) {
            case 0:
                app->current_stack = (app->current_stack - 1 + app->stack_count) % app->stack_count;
                break;
            case 1:
                ps->answer_style = (ps->answer_style + 1) % 2;
                break;
            case 2:
                if (ps->num_choices > 2) ps->num_choices--;
                break;
            case 3:
                if (ps->range_min > 1) ps->range_min--;
                break;
            case 4:
                if (ps->range_max > ps->range_min) ps->range_max--;
                break;
            case 5:
                ps->card_filter = (ps->card_filter - 1 + FILTER_COUNT) % FILTER_COUNT;
                break;
            case 6:
                ps->limit_mode = (ps->limit_mode - 1 + LIMIT_COUNT) % LIMIT_COUNT;
                break;
            case 7:
                if (ps->limit_mode == LIMIT_TIME && ps->limit_value > 10) ps->limit_value -= 10;
                else if (ps->limit_mode != LIMIT_NONE && ps->limit_value > 1) ps->limit_value--;
                break;
            case 8:
                ps->show_mnemonic = !ps->show_mnemonic;
                break;
            case 9:
                ps->randomize = !ps->randomize;
                break;
            }
            break;
        case '\n': case KEY_ENTER:
            app->settings_sel = sel;
            return SCREEN_PLAY;
        case 27: case 'q':
            app->settings_sel = sel;
            return SCREEN_PLAY;
        }
    }
}

/* ─── Study Screen ───────────────────────────────────────────── */

int screen_study(App *app)
{
    if (app->stack_count == 0) return SCREEN_MENU;

    Stack *st = &app->stacks[app->current_stack];
    int pos = app->study_pos;
    int show_mn = app->study_show_mnemonic;

    for (;;) {
        erase();

        char title[128];
        snprintf(title, sizeof(title), "STUDY: %s", st->name);
        ui_draw_title_bar(title);

        StackEntry *e = &st->entries[pos];

        /* position indicator */
        char posbar[64];
        snprintf(posbar, sizeof(posbar), "Position %d of %d", e->position, st->count);
        ui_draw_centered(3, posbar, COLOR_PAIR(CP_DIM));

        /* progress bar */
        int barw = COLS / 2;
        int bx = (COLS - barw) / 2;
        int filled = (pos * barw) / (st->count > 1 ? st->count - 1 : 1);
        mvaddch(4, bx - 1, '[');
        for (int i = 0; i < barw; i++) {
            if (i <= filled)
                mvaddch(4, bx + i, '=' | COLOR_PAIR(CP_CORRECT));
            else
                mvaddch(4, bx + i, '-' | COLOR_PAIR(CP_DIM));
        }
        mvaddch(4, bx + barw, ']');

        /* card display - big and centered */
        int cy = LINES / 2 - 4;
        if (cy < 7) cy = 7;

        char card_disp[16];
        card_display(&e->card, card_disp, sizeof(card_disp));

        /* big number */
        char posnum[16];
        snprintf(posnum, sizeof(posnum), "#%d", e->position);
        attron(COLOR_PAIR(CP_SCORE) | A_BOLD);
        ui_draw_centered(cy, posnum, 0);
        attroff(COLOR_PAIR(CP_SCORE) | A_BOLD);

        /* card with color */
        int cp = card_color_pair(&e->card);
        attron(COLOR_PAIR(cp) | A_BOLD);
        char full_card[64];
        snprintf(full_card, sizeof(full_card), "%s  %s of %s",
                 card_disp, card_rank_name(e->card.rank), card_suit_name(e->card.suit));
        ui_draw_centered(cy + 2, full_card, 0);
        attroff(COLOR_PAIR(cp) | A_BOLD);

        /* mnemonic */
        if (show_mn && e->mnemonic[0]) {
            ui_draw_centered(cy + 5, e->mnemonic, COLOR_PAIR(CP_DIM));
        } else if (e->mnemonic[0]) {
            ui_draw_centered(cy + 5, "[Press Space to reveal mnemonic]", COLOR_PAIR(CP_DIM));
        }

        /* navigation hints at sides */
        if (pos > 0) {
            mvprintw(cy + 2, 2, "<");
        }
        if (pos < st->count - 1) {
            mvprintw(cy + 2, COLS - 3, ">");
        }

        ui_draw_help_bar("h/l or arrows: prev/next  Space: toggle mnemonic  g: jump  Esc: back");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'h': case KEY_LEFT: case KEY_UP: case 'k':
            if (pos > 0) pos--;
            show_mn = 0;
            break;
        case 'l': case KEY_RIGHT: case KEY_DOWN: case 'j':
            if (pos < st->count - 1) pos++;
            show_mn = 0;
            break;
        case ' ':
            show_mn = !show_mn;
            break;
        case KEY_HOME: case '0':
            pos = 0;
            show_mn = 0;
            break;
        case KEY_END: case '$':
            pos = st->count - 1;
            show_mn = 0;
            break;
        case 'g': case '/': {
            /* jump to position */
            curs_set(1);
            char input[8] = {0};
            int ilen = 0;
            mvprintw(LINES - 3, COLS / 2 - 15, "Jump to position: ");
            refresh();
            int ic;
            while ((ic = getch()) != '\n' && ic != KEY_ENTER && ic != 27) {
                if ((ic == KEY_BACKSPACE || ic == 127 || ic == 8) && ilen > 0) {
                    input[--ilen] = '\0';
                } else if (ic >= '0' && ic <= '9' && ilen < 3) {
                    input[ilen++] = ic;
                    input[ilen] = '\0';
                }
                mvprintw(LINES - 3, COLS / 2 + 3, "%-5s", input);
                refresh();
            }
            curs_set(0);
            if (ic != 27 && ilen > 0) {
                int target = atoi(input);
                if (target >= 1 && target <= st->count) {
                    pos = target - 1;
                    show_mn = 0;
                }
            }
            break;
        }
        case 27: case 'q':
            app->study_pos = pos;
            app->study_show_mnemonic = show_mn;
            return SCREEN_MENU;
        }
    }
}

/* ─── Stacks Screen ──────────────────────────────────────────── */

int screen_stacks(App *app)
{
    for (;;) {
        erase();
        ui_draw_title_bar("STACKS");

        int y = 4;
        for (int i = 0; i < app->stack_count; i++) {
            Stack *s = &app->stacks[i];
            char errbuf[128];
            int valid = (stack_validate(s, errbuf, sizeof(errbuf)) == 0);

            if (i == app->stack_sel) {
                attron(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
                mvprintw(y + i * 2, COLS / 2 - 25,
                         "  > %-20s  %d cards  %s  %s",
                         s->name, s->count,
                         s->builtin ? "[built-in]" : "[custom]",
                         valid ? "" : "[INVALID]");
                attroff(COLOR_PAIR(CP_HIGHLIGHT) | A_BOLD);
            } else {
                mvprintw(y + i * 2, COLS / 2 - 25,
                         "    %-20s  %d cards  %s  %s",
                         s->name, s->count,
                         s->builtin ? "[built-in]" : "[custom]",
                         valid ? "" : "[INVALID]");
            }

            if (i == app->current_stack) {
                attron(COLOR_PAIR(CP_CORRECT));
                printw("  *active*");
                attroff(COLOR_PAIR(CP_CORRECT));
            }
        }

        if (app->stack_count == 0) {
            ui_draw_centered(LINES / 2, "No stacks found!", COLOR_PAIR(CP_WRONG));
        }

        ui_draw_help_bar("j/k: navigate  Enter: view  a: set active  v: validate  Esc: back");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            if (app->stack_count > 0)
                app->stack_sel = (app->stack_sel - 1 + app->stack_count) % app->stack_count;
            break;
        case 'j': case KEY_DOWN:
            if (app->stack_count > 0)
                app->stack_sel = (app->stack_sel + 1) % app->stack_count;
            break;
        case '\n': case KEY_ENTER:
            if (app->stack_count > 0) return SCREEN_STACK_VIEW;
            break;
        case 'a':
            if (app->stack_count > 0)
                app->current_stack = app->stack_sel;
            break;
        case 'v':
            if (app->stack_count > 0) {
                char errbuf[256];
                Stack *s = &app->stacks[app->stack_sel];
                if (stack_validate(s, errbuf, sizeof(errbuf)) == 0) {
                    attron(COLOR_PAIR(CP_CORRECT));
                    ui_draw_centered(LINES - 3, "Stack is valid!", 0);
                    attroff(COLOR_PAIR(CP_CORRECT));
                } else {
                    attron(COLOR_PAIR(CP_WRONG));
                    ui_draw_centered(LINES - 3, errbuf, 0);
                    attroff(COLOR_PAIR(CP_WRONG));
                }
                refresh();
                getch();
            }
            break;
        case 27: case 'q':
            return SCREEN_MENU;
        }
    }
}

/* ─── Stack View ─────────────────────────────────────────────── */

int screen_stack_view(App *app)
{
    if (app->stack_count == 0) return SCREEN_STACKS;

    Stack *st = &app->stacks[app->stack_sel];
    int scroll = 0;
    int sel = 0;

    for (;;) {
        erase();

        char title[128];
        snprintf(title, sizeof(title), "STACK: %s (%d cards)", st->name, st->count);
        ui_draw_title_bar(title);

        /* header */
        attron(A_BOLD | A_UNDERLINE);
        mvprintw(2, 4, "%-6s %-8s %-20s %s", "Pos", "Card", "Full Name", "Mnemonic");
        attroff(A_BOLD | A_UNDERLINE);

        int visible = LINES - 6;
        if (visible < 1) visible = 1;

        for (int i = 0; i < visible && (scroll + i) < st->count; i++) {
            int idx = scroll + i;
            StackEntry *e = &st->entries[idx];
            char code[8], disp[16];
            card_code(&e->card, code, sizeof(code));
            card_display(&e->card, disp, sizeof(disp));

            int y = 3 + i;
            if (idx == sel) {
                attron(COLOR_PAIR(CP_HIGHLIGHT));
            }

            char full[32];
            snprintf(full, sizeof(full), "%s of %s",
                     card_rank_name(e->card.rank), card_suit_name(e->card.suit));

            mvprintw(y, 4, "%-6d", e->position);

            if (idx == sel) attroff(COLOR_PAIR(CP_HIGHLIGHT));

            /* card with suit color */
            int cp = card_color_pair(&e->card);
            attron(COLOR_PAIR(cp) | A_BOLD);
            mvprintw(y, 11, "%-8s", disp);
            attroff(COLOR_PAIR(cp) | A_BOLD);

            if (idx == sel) attron(COLOR_PAIR(CP_HIGHLIGHT));
            mvprintw(y, 19, "%-20s", full);
            if (idx == sel) attroff(COLOR_PAIR(CP_HIGHLIGHT));

            if (e->mnemonic[0]) {
                attron(COLOR_PAIR(CP_DIM));
                mvprintw(y, 40, "%.38s", e->mnemonic);
                attroff(COLOR_PAIR(CP_DIM));
            }
        }

        /* scrollbar indicator */
        if (st->count > visible) {
            char sbar[32];
            snprintf(sbar, sizeof(sbar), "[%d-%d of %d]",
                     scroll + 1, scroll + visible, st->count);
            mvprintw(LINES - 2, COLS - 20, "%s", sbar);
        }

        ui_draw_help_bar("j/k: scroll  g: jump  Esc: back to stacks");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            if (sel > 0) {
                sel--;
                if (sel < scroll) scroll = sel;
            }
            break;
        case 'j': case KEY_DOWN:
            if (sel < st->count - 1) {
                sel++;
                if (sel >= scroll + visible) scroll = sel - visible + 1;
            }
            break;
        case KEY_PPAGE:
            sel -= visible;
            if (sel < 0) sel = 0;
            scroll = sel;
            break;
        case KEY_NPAGE:
            sel += visible;
            if (sel >= st->count) sel = st->count - 1;
            scroll = sel - visible + 1;
            if (scroll < 0) scroll = 0;
            break;
        case KEY_HOME: case 'g':
            sel = 0;
            scroll = 0;
            break;
        case KEY_END: case 'G':
            sel = st->count - 1;
            scroll = st->count - visible;
            if (scroll < 0) scroll = 0;
            break;
        case 27: case 'q':
            return SCREEN_STACKS;
        }
    }
}

/* ─── Progress Screen ────────────────────────────────────────── */

int screen_progress(App *app)
{
    Progress *p = &app->progress;

    for (;;) {
        erase();
        ui_draw_title_bar("PROGRESS");

        int y = 4;
        int lx = COLS / 2 - 20;
        int vx = COLS / 2 + 5;

        attron(A_BOLD);
        mvprintw(y, lx, "Total Sessions:");
        attroff(A_BOLD);
        mvprintw(y, vx, "%d", p->total_sessions);
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Total Correct:");
        attroff(A_BOLD);
        attron(COLOR_PAIR(CP_CORRECT));
        mvprintw(y, vx, "%d", p->total_correct);
        attroff(COLOR_PAIR(CP_CORRECT));
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Total Incorrect:");
        attroff(A_BOLD);
        attron(COLOR_PAIR(CP_WRONG));
        mvprintw(y, vx, "%d", p->total_incorrect);
        attroff(COLOR_PAIR(CP_WRONG));
        y += 2;

        int total = p->total_correct + p->total_incorrect;
        int pct = total > 0 ? (p->total_correct * 100 / total) : 0;
        attron(A_BOLD);
        mvprintw(y, lx, "Overall Accuracy:");
        attroff(A_BOLD);
        mvprintw(y, vx, "%d%%", pct);
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Best Score:");
        attroff(A_BOLD);
        attron(COLOR_PAIR(CP_SCORE));
        mvprintw(y, vx, "%d%%", p->best_score);
        attroff(COLOR_PAIR(CP_SCORE));
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Day Streak:");
        attroff(A_BOLD);
        attron(COLOR_PAIR(CP_SCORE) | A_BOLD);
        mvprintw(y, vx, "%d days", p->current_streak);
        attroff(COLOR_PAIR(CP_SCORE) | A_BOLD);
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Best Streak:");
        attroff(A_BOLD);
        mvprintw(y, vx, "%d days", p->best_streak);
        y += 2;

        attron(A_BOLD);
        mvprintw(y, lx, "Last Practice:");
        attroff(A_BOLD);
        mvprintw(y, vx, "%s", p->last_date[0] ? p->last_date : "Never");
        y += 3;

        /* hardest cards */
        mvhline(y, COLS / 4, ACS_HLINE, COLS / 2);
        y += 1;
        attron(A_BOLD | COLOR_PAIR(CP_TITLE));
        ui_draw_centered(y, "Hardest Positions (most errors)", 0);
        attroff(A_BOLD | COLOR_PAIR(CP_TITLE));
        y += 2;

        /* find top 5 error positions */
        int top[5] = {-1, -1, -1, -1, -1};
        for (int i = 0; i < STACK_SIZE; i++) {
            for (int j = 0; j < 5; j++) {
                if (top[j] == -1 || p->card_errors[i] > p->card_errors[top[j]]) {
                    for (int k = 4; k > j; k--) top[k] = top[k-1];
                    top[j] = i;
                    break;
                }
            }
        }

        if (app->stack_count > 0) {
            Stack *st = &app->stacks[app->current_stack];
            for (int i = 0; i < 5; i++) {
                if (top[i] >= 0 && top[i] < st->count && p->card_errors[top[i]] > 0) {
                    char disp[16];
                    card_display(&st->entries[top[i]].card, disp, sizeof(disp));
                    int cp = card_color_pair(&st->entries[top[i]].card);
                    mvprintw(y + i, lx, "#%-3d", st->entries[top[i]].position);
                    attron(COLOR_PAIR(cp) | A_BOLD);
                    printw(" %-6s", disp);
                    attroff(COLOR_PAIR(cp) | A_BOLD);
                    attron(COLOR_PAIR(CP_WRONG));
                    printw("  %d errors", p->card_errors[top[i]]);
                    attroff(COLOR_PAIR(CP_WRONG));
                }
            }
        }

        ui_draw_help_bar("r: reset progress  Esc: back");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'r': {
            ui_draw_centered(LINES - 3, "Reset all progress? (y/n)", COLOR_PAIR(CP_WRONG) | A_BOLD);
            refresh();
            int confirm = getch();
            if (confirm == 'y' || confirm == 'Y') {
                progress_reset(app);
            }
            break;
        }
        case 27: case 'q':
            return SCREEN_MENU;
        }
    }
}

/* ─── Learn Screen ───────────────────────────────────────────── */

int screen_learn(App *app)
{
    /* load lesson file */
    char path[MAX_PATH];
    snprintf(path, sizeof(path), "%s/../lessons/learn-method.txt", app->data_dir);

    FILE *f = fopen(path, "r");
    if (!f) {
        /* try alternate path */
        snprintf(path, sizeof(path), "%s/data/lessons/learn-method.txt", app->data_dir);
        f = fopen(path, "r");
    }

    char lines[200][256];
    int line_count = 0;

    if (f) {
        while (line_count < 200 && fgets(lines[line_count], sizeof(lines[0]), f)) {
            /* strip trailing newline */
            int len = strlen(lines[line_count]);
            if (len > 0 && lines[line_count][len-1] == '\n')
                lines[line_count][len-1] = '\0';
            line_count++;
        }
        fclose(f);
    } else {
        strcpy(lines[0], "Learn content not found.");
        strcpy(lines[1], "Check data/lessons/learn-method.txt");
        line_count = 2;
    }

    int scroll = app->learn_scroll;
    int visible = LINES - 4;

    for (;;) {
        erase();
        ui_draw_title_bar("LEARN THE METHOD");

        for (int i = 0; i < visible && (scroll + i) < line_count; i++) {
            char *line = lines[scroll + i];
            int y = 2 + i;

            if (line[0] == '#' && line[1] == '#' && line[2] == '#') {
                attron(COLOR_PAIR(CP_SCORE) | A_BOLD);
                mvprintw(y, 4, "%s", line + 4);
                attroff(COLOR_PAIR(CP_SCORE) | A_BOLD);
            } else if (line[0] == '#' && line[1] == '#') {
                attron(COLOR_PAIR(CP_TITLE) | A_BOLD);
                mvprintw(y, 2, "%s", line + 3);
                attroff(COLOR_PAIR(CP_TITLE) | A_BOLD);
            } else if (line[0] == '#') {
                attron(COLOR_PAIR(CP_TITLE) | A_BOLD | A_UNDERLINE);
                mvprintw(y, 0, "%s", line + 2);
                attroff(COLOR_PAIR(CP_TITLE) | A_BOLD | A_UNDERLINE);
            } else {
                mvprintw(y, 4, "%s", line);
            }
        }

        /* scroll indicator */
        if (line_count > visible) {
            char sbar[32];
            snprintf(sbar, sizeof(sbar), "[%d/%d]", scroll + 1, line_count);
            mvprintw(LINES - 2, COLS - 15, "%s", sbar);
        }

        ui_draw_help_bar("j/k or arrows: scroll  Home/End: top/bottom  Esc: back");
        refresh();

        int ch = getch();
        switch (ch) {
        case 'k': case KEY_UP:
            if (scroll > 0) scroll--;
            break;
        case 'j': case KEY_DOWN:
            if (scroll < line_count - visible) scroll++;
            break;
        case KEY_PPAGE:
            scroll -= visible;
            if (scroll < 0) scroll = 0;
            break;
        case KEY_NPAGE:
            scroll += visible;
            if (scroll > line_count - visible) scroll = line_count - visible;
            if (scroll < 0) scroll = 0;
            break;
        case KEY_HOME: case 'g':
            scroll = 0;
            break;
        case KEY_END: case 'G':
            scroll = line_count - visible;
            if (scroll < 0) scroll = 0;
            break;
        case 27: case 'q':
            app->learn_scroll = scroll;
            return SCREEN_MENU;
        }
    }
}

/* ─── Path Setup ─────────────────────────────────────────────── */

static void setup_paths(App *app, const char *argv0)
{
    /* find data dir relative to binary */
    char resolved[MAX_PATH] = {0};

    /* try to resolve from argv0 */
    if (argv0[0] == '/') {
        strncpy(resolved, argv0, sizeof(resolved) - 1);
    } else {
        /* try relative to cwd */
        char cwd[MAX_PATH];
        if (getcwd(cwd, sizeof(cwd))) {
            snprintf(resolved, sizeof(resolved), "%s/%s", cwd, argv0);
        }
    }

    /* strip binary name to get bin dir, then go up for data dir */
    char *slash = strrchr(resolved, '/');
    if (slash) {
        *slash = '\0';
        /* we're in bin/ or src/, go to data/stacks */
        char *binslash = strrchr(resolved, '/');
        if (binslash) {
            *binslash = '\0';
            snprintf(app->data_dir, sizeof(app->data_dir), "%s/data/stacks", resolved);
        }
    }

    /* fallback: try common locations */
    struct stat st;
    if (stat(app->data_dir, &st) != 0) {
        /* try relative to cwd */
        char cwd[MAX_PATH];
        if (getcwd(cwd, sizeof(cwd))) {
            snprintf(app->data_dir, sizeof(app->data_dir), "%s/data/stacks", cwd);
        }
    }
    if (stat(app->data_dir, &st) != 0) {
        /* try MEMDECK_DATA env */
        const char *env = getenv("MEMDECK_DATA");
        if (env) {
            snprintf(app->data_dir, sizeof(app->data_dir), "%s/stacks", env);
        }
    }

    /* user data dir (XDG compliant) */
    const char *xdg = getenv("XDG_DATA_HOME");
    if (xdg) {
        snprintf(app->user_dir, sizeof(app->user_dir), "%s/memdeck", xdg);
    } else {
        const char *home = getenv("HOME");
        if (home) {
            snprintf(app->user_dir, sizeof(app->user_dir), "%s/.local/share/memdeck", home);
        } else {
            strncpy(app->user_dir, "/tmp/memdeck", sizeof(app->user_dir) - 1);
        }
    }

    /* ensure user dirs exist */
    mkdir(app->user_dir, 0755);
    char user_stacks[MAX_PATH];
    snprintf(user_stacks, sizeof(user_stacks), "%s/stacks", app->user_dir);
    mkdir(user_stacks, 0755);

    snprintf(app->progress_file, sizeof(app->progress_file),
             "%s/progress.dat", app->user_dir);
}

/* ─── Default Settings ───────────────────────────────────────── */

static void init_defaults(App *app)
{
    memset(app, 0, sizeof(*app));

    app->settings.question_type = Q_POS_TO_CARD;
    app->settings.answer_style = ANSWER_MCQ;
    app->settings.num_choices = DEFAULT_CHOICES;
    app->settings.range_min = 1;
    app->settings.range_max = 52;
    app->settings.limit_mode = LIMIT_QUESTIONS;
    app->settings.limit_value = 20;
    app->settings.card_filter = FILTER_ALL;
    app->settings.show_mnemonic = 1;
    app->settings.randomize = 1;

    app->screen = SCREEN_MENU;
}

/* ─── Main ───────────────────────────────────────────────────── */

int main(int argc, char **argv)
{
    App app;
    init_defaults(&app);
    setup_paths(&app, argv[0]);

    /* non-interactive commands */
    if (argc >= 2) {
        if (strcmp(argv[1], "validate") == 0) {
            if (argc < 3) {
                fprintf(stderr, "Usage: memdeck validate <stack-file>\n");
                return 1;
            }
            Stack s;
            if (stack_load(&s, argv[2]) != 0) {
                fprintf(stderr, "Error: cannot load %s\n", argv[2]);
                return 1;
            }
            char errbuf[256];
            if (stack_validate(&s, errbuf, sizeof(errbuf)) != 0) {
                fprintf(stderr, "INVALID: %s\n", errbuf);
                return 1;
            }
            printf("OK: %s is valid (%d cards)\n", s.name, s.count);
            return 0;
        }

        if (strcmp(argv[1], "stats") == 0) {
            progress_load(&app);
            Progress *p = &app.progress;
            int total = p->total_correct + p->total_incorrect;
            printf("Sessions:   %d\n", p->total_sessions);
            printf("Correct:    %d\n", p->total_correct);
            printf("Incorrect:  %d\n", p->total_incorrect);
            printf("Accuracy:   %d%%\n", total > 0 ? p->total_correct * 100 / total : 0);
            printf("Best Score: %d%%\n", p->best_score);
            printf("Day Streak: %d\n", p->current_streak);
            printf("Last Date:  %s\n", p->last_date[0] ? p->last_date : "Never");
            return 0;
        }

        if (strcmp(argv[1], "reset-progress") == 0) {
            progress_load(&app);
            progress_reset(&app);
            printf("Progress reset.\n");
            return 0;
        }

        if (strcmp(argv[1], "export") == 0) {
            if (argc < 3) {
                fprintf(stderr, "Usage: memdeck export <stack-name>\n");
                return 1;
            }
            stack_discover(&app);
            for (int i = 0; i < app.stack_count; i++) {
                if (strcasecmp(app.stacks[i].name, argv[2]) == 0) {
                    Stack *s = &app.stacks[i];
                    printf("# %s\n# position\tcard\tmnemonic\n", s->name);
                    for (int j = 0; j < s->count; j++) {
                        char code[8];
                        card_code(&s->entries[j].card, code, sizeof(code));
                        if (s->entries[j].mnemonic[0])
                            printf("%d\t%s\t%s\n", s->entries[j].position, code, s->entries[j].mnemonic);
                        else
                            printf("%d\t%s\n", s->entries[j].position, code);
                    }
                    return 0;
                }
            }
            fprintf(stderr, "Stack '%s' not found.\n", argv[2]);
            return 1;
        }

        if (strcmp(argv[1], "list") == 0 || strcmp(argv[1], "stacks") == 0) {
            stack_discover(&app);
            for (int i = 0; i < app.stack_count; i++) {
                printf("%-20s %d cards  %s  %s\n",
                       app.stacks[i].name,
                       app.stacks[i].count,
                       app.stacks[i].builtin ? "[built-in]" : "[custom]",
                       app.stacks[i].filename);
            }
            return 0;
        }

        if (strcmp(argv[1], "help") == 0 || strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
            printf("memdeck - Memorized Deck Trainer\n\n");
            printf("Usage: memdeck [command]\n\n");
            printf("Commands:\n");
            printf("  (none)           Launch interactive TUI\n");
            printf("  play             Launch directly into practice\n");
            printf("  study            Launch directly into study mode\n");
            printf("  stacks           List available stacks\n");
            printf("  validate <file>  Validate a stack file\n");
            printf("  export <name>    Export a stack to stdout\n");
            printf("  stats            Show progress statistics\n");
            printf("  reset-progress   Reset all progress data\n");
            printf("  help             Show this help\n");
            return 0;
        }
    }

    /* interactive TUI mode */
    stack_discover(&app);
    if (app.stack_count == 0) {
        fprintf(stderr, "Error: no stacks found in %s\n", app.data_dir);
        fprintf(stderr, "Set MEMDECK_DATA to the data directory or run from the project root.\n");
        return 1;
    }

    progress_load(&app);
    ui_init();

    /* check for direct-launch modes */
    int start_screen = SCREEN_MENU;
    if (argc >= 2) {
        if (strcmp(argv[1], "play") == 0) start_screen = SCREEN_PLAY;
        else if (strcmp(argv[1], "study") == 0) start_screen = SCREEN_STUDY;
    }
    app.screen = start_screen;

    /* main loop */
    int running = 1;
    while (running) {
        int next;
        switch (app.screen) {
        case SCREEN_MENU:       next = screen_menu(&app); break;
        case SCREEN_PLAY:       next = screen_play_menu(&app); break;
        case SCREEN_PRACTICE:   next = screen_practice(&app); break;
        case SCREEN_COMPLETE:   next = screen_complete(&app); break;
        case SCREEN_SETTINGS:   next = screen_settings(&app); break;
        case SCREEN_STUDY:      next = screen_study(&app); break;
        case SCREEN_STACKS:     next = screen_stacks(&app); break;
        case SCREEN_STACK_VIEW: next = screen_stack_view(&app); break;
        case SCREEN_PROGRESS:   next = screen_progress(&app); break;
        case SCREEN_LEARN:      next = screen_learn(&app); break;
        case SCREEN_QUIT:       running = 0; continue;
        default:                next = SCREEN_MENU; break;
        }
        app.screen = next;
    }

    progress_save(&app);
    ui_cleanup();
    return 0;
}
