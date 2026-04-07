#ifndef MEMDECK_H
#define MEMDECK_H

#include <ncursesw/curses.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <time.h>
#include <ctype.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>
#include <locale.h>

#define MAX_STACKS      32
#define STACK_SIZE      52
#define MAX_NAME        64
#define MAX_PATH        1024
#define MAX_MNEMONIC    256
#define MAX_CHOICES     6
#define DEFAULT_CHOICES 4

/* Suits */
#define SUIT_SPADES   0
#define SUIT_HEARTS   1
#define SUIT_CLUBS    2
#define SUIT_DIAMONDS 3

/* Card filter modes */
#define FILTER_ALL       0
#define FILTER_BLACK     1
#define FILTER_RED       2
#define FILTER_HEARTS    3
#define FILTER_SPADES    4
#define FILTER_CLUBS     5
#define FILTER_DIAMONDS  6
#define FILTER_FACE      7
#define FILTER_NUMBERS   8

/* Question types */
#define Q_POS_TO_CARD    0
#define Q_CARD_TO_POS    1
#define Q_NEXT_CARD      2
#define Q_PREV_CARD      3
#define Q_SUIT_DRILL     4
#define Q_VALUE_DRILL    5
#define Q_MIXED          6
#define Q_TYPE_COUNT     7

/* Limit modes */
#define LIMIT_NONE       0
#define LIMIT_TIME       1
#define LIMIT_QUESTIONS  2
#define LIMIT_LIVES      3

/* Answer styles */
#define ANSWER_MCQ       0
#define ANSWER_FREE      1

/* Screens */
#define SCREEN_MENU       0
#define SCREEN_PLAY       1
#define SCREEN_PRACTICE   2
#define SCREEN_COMPLETE   3
#define SCREEN_SETTINGS   4
#define SCREEN_STUDY      5
#define SCREEN_STACKS     6
#define SCREEN_STACK_VIEW 7
#define SCREEN_PROGRESS   8
#define SCREEN_LEARN      9
#define SCREEN_QUIT      99

/* Color pairs */
#define CP_NORMAL    1
#define CP_TITLE     2
#define CP_HIGHLIGHT 3
#define CP_CORRECT   4
#define CP_WRONG     5
#define CP_HEARTS    6
#define CP_DIAMONDS  7
#define CP_SPADES    8
#define CP_CLUBS     9
#define CP_HELP      10
#define CP_SCORE     11
#define CP_DIM       12
#define CP_SELECTED  13
#define CP_RAINBOW0  14  /* red */
#define CP_RAINBOW1  15  /* yellow */
#define CP_RAINBOW2  16  /* green */
#define CP_RAINBOW3  17  /* cyan */
#define CP_RAINBOW4  18  /* blue */
#define CP_RAINBOW5  19  /* magenta */
#define CP_RAINBOW_COUNT 6

typedef struct {
    int rank; /* 1=A, 2-10, 11=J, 12=Q, 13=K */
    int suit; /* SUIT_* constants */
} Card;

typedef struct {
    int position;
    Card card;
    char mnemonic[MAX_MNEMONIC];
} StackEntry;

typedef struct {
    char name[MAX_NAME];
    char filename[MAX_PATH];
    StackEntry entries[STACK_SIZE];
    int count;
    int builtin;
} Stack;

typedef struct {
    int question_type;
    int answer_style;
    int num_choices;
    int range_min;
    int range_max;
    int limit_mode;
    int limit_value;
    int card_filter;
    int show_mnemonic;
    int randomize;
} PracticeSettings;

typedef struct {
    int correct;
    int incorrect;
    int streak;
    int best_streak;
    int lives;
    int questions_asked;
    time_t start_time;
    int active;
    int current_question;
    int current_answer;
    int choices[MAX_CHOICES];
    int selected;
    int answered;
    int last_correct;
    char question_text[256];
    char input_buf[32];
    int input_len;
    int filtered_indices[STACK_SIZE];
    int filtered_count;
} Session;

typedef struct {
    int total_sessions;
    int total_correct;
    int total_incorrect;
    int best_score;
    int current_streak;
    int best_streak;
    char last_date[16];
    int card_errors[STACK_SIZE];
    int card_correct[STACK_SIZE];
} Progress;

typedef struct {
    Stack stacks[MAX_STACKS];
    int stack_count;
    int current_stack;
    PracticeSettings settings;
    Session session;
    Progress progress;
    int screen;
    int menu_sel;
    int play_sel;
    int stack_sel;
    int study_pos;
    int study_show_mnemonic;
    int learn_scroll;
    int settings_sel;
    char data_dir[MAX_PATH];
    char user_dir[MAX_PATH];
    char progress_file[MAX_PATH];
} App;

/* card.c */
int card_parse(const char *s, Card *c);
void card_code(const Card *c, char *buf, int buflen);
void card_display(const Card *c, char *buf, int buflen);
const char *card_rank_name(int rank);
const char *card_suit_name(int suit);
const char *card_suit_symbol(int suit);
int card_color_pair(const Card *c);
int card_equal(const Card *a, const Card *b);
int card_matches_filter(const Card *c, int filter);

/* stack.c */
int stack_load(Stack *s, const char *path);
int stack_save(const Stack *s, const char *path);
int stack_validate(const Stack *s, char *errbuf, int errlen);
void stack_discover(App *app);

/* progress.c */
void progress_load(App *app);
void progress_save(const App *app);
void progress_update(App *app, int correct, int position);
void progress_reset(App *app);
const char *progress_today(void);

/* ui.c */
#define CARD_ART_W 13
#define CARD_ART_H 9

void ui_init(void);
void ui_cleanup(void);
void ui_draw_box(int y, int x, int h, int w, const char *title);
void ui_draw_centered(int y, const char *text, int attr);
void ui_draw_help_bar(const char *text);
void ui_draw_title_bar(const char *text);
void ui_draw_card_fancy(int y, int x, const Card *c);
void ui_draw_card_art(int y, int x, const Card *c);
void ui_draw_card_back(int y, int x);

/* screens */
int screen_menu(App *app);
int screen_play_menu(App *app);
int screen_practice(App *app);
int screen_complete(App *app);
int screen_settings(App *app);
int screen_study(App *app);
int screen_stacks(App *app);
int screen_stack_view(App *app);
int screen_progress(App *app);
int screen_learn(App *app);

/* session */
void session_init(App *app);
void session_generate_question(App *app);
void session_check_answer(App *app, int choice);
int session_is_over(App *app);
void session_build_filter(App *app);

/* sound.c */
void sound_success(void);
void sound_fail(void);
void sound_set_data_dir(const char *dir);
void sound_music_start(void);
void sound_music_stop(void);
int  sound_music_source(void); /* 0=none, 1=abc, 2=hardcoded */

/* abc.c — ABC notation parser and PCM generator */
#define ABC_MAX_VOICES  8
#define ABC_MAX_NOTES   1024
#define SAMPLE_RATE_ABC 22050

typedef struct {
    char name[32];
    int amplitude;        /* per-voice amplitude (0-127) */
    int staccato;         /* 1 = staccato (3/4 length), 0 = legato (9/10 length) */
    double freqs[ABC_MAX_NOTES]; /* frequency per step (0 = rest) */
    int note_count;
} AbcVoice;

typedef struct {
    char title[128];
    int bpm;
    int step_ms;          /* duration of one default-length note in ms */
    AbcVoice voices[ABC_MAX_VOICES];
    int voice_count;
} AbcMusic;

int abc_load(const char *path, AbcMusic *music);
int abc_load_voices(const char *paths[], int path_count, AbcMusic *music);
unsigned char *abc_generate_pcm(const AbcMusic *music, int *out_len);

#endif
