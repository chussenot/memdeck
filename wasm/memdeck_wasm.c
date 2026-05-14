/*
 * memdeck_wasm.c — WebAssembly wrapper for MemDeck
 *
 * Compiles the core C modules (card, stack, session, progress) to WASM via
 * Emscripten and exposes a JSON-based API for the JavaScript front-end.
 *
 * Sound and ncurses UI are intentionally excluded; the browser handles them.
 */

#include "../src/memdeck.h"
#include <emscripten.h>
#include <time.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Global state ─────────────────────────────────────────────────────────── */

static App  g_app;
static char g_json[65536];   /* scratch buffer for returned JSON strings */

/* ── Sound stubs (browser handles audio via Web Audio API) ───────────────── */

void sound_success(void)                    {}
void sound_fail(void)                       {}
void sound_music_start(void)                {}
void sound_music_stop(void)                 {}
void sound_set_data_dir(const char *d)      { (void)d; }
int  sound_music_source(void)               { return 0; }
const char *sound_music_title(void)         { return ""; }

/* ── JSON helpers ─────────────────────────────────────────────────────────── */

static void json_escape(const char *src, char *dst, int dstlen)
{
    int d = 0;
    for (int i = 0; src[i] && d < dstlen - 3; i++) {
        unsigned char c = (unsigned char)src[i];
        if (c == '"' || c == '\\') { dst[d++] = '\\'; dst[d++] = (char)c; }
        else if (c == '\n')        { dst[d++] = '\\'; dst[d++] = 'n'; }
        else if (c == '\r')        { dst[d++] = '\\'; dst[d++] = 'r'; }
        else                       { dst[d++] = (char)c; }
    }
    dst[d] = '\0';
}

/* Append a card entry as JSON object into buf at offset *pos */
static int entry_json(char *buf, int buflen, int *pos,
                       const App *app, int stack_idx, int entry_idx)
{
    if (stack_idx < 0 || stack_idx >= app->stack_count) return -1;
    const Stack *s = &app->stacks[stack_idx];
    if (entry_idx < 0 || entry_idx >= s->count) return -1;
    const StackEntry *e = &s->entries[entry_idx];

    char code[8], disp[16], mne[MAX_MNEMONIC * 2 + 4];
    card_code(&e->card, code, sizeof(code));
    card_display(&e->card, disp, sizeof(disp));
    json_escape(e->mnemonic, mne, sizeof(mne));

    *pos += snprintf(buf + *pos, buflen - *pos,
        "{\"idx\":%d,\"pos\":%d,\"rank\":%d,\"suit\":%d,"
        "\"code\":\"%s\",\"display\":\"%s\","
        "\"rank_name\":\"%s\",\"suit_name\":\"%s\","
        "\"suit_symbol\":\"%s\",\"mnemonic\":\"%s\"}",
        entry_idx, e->position, e->card.rank, e->card.suit,
        code, disp,
        card_rank_name(e->card.rank),
        card_suit_name(e->card.suit),
        card_suit_symbol(e->card.suit),
        mne);
    return 0;
}

/* ── TSV parser (string-based, mirrors stack_load logic) ─────────────────── */

static int parse_stack_tsv(Stack *s, const char *tsv)
{
    s->count = 0;
    const char *p = tsv;

    while (*p && s->count < STACK_SIZE) {
        const char *eol = p;
        while (*eol && *eol != '\n') eol++;

        int len = (int)(eol - p);
        if (len >= 1023) len = 1023;
        char line[1024];
        memcpy(line, p, len);
        line[len] = '\0';
        p = (*eol == '\n') ? eol + 1 : eol;

        char *lp = line;
        while (*lp == ' ' || *lp == '\t') lp++;
        if (*lp == '#' || *lp == '\0') continue;

        char *f1 = strtok(lp, "\t\n");
        char *f2 = strtok(NULL, "\t\n");
        char *f3 = strtok(NULL, "\n");
        if (!f1 || !f2) continue;

        int pos = atoi(f1);
        if (pos < 1 || pos > 52) continue;

        char cardstr[16] = {0};
        strncpy(cardstr, f2, sizeof(cardstr) - 1);
        int clen = strlen(cardstr);
        while (clen > 0 && (cardstr[clen-1] == ' ' || cardstr[clen-1] == '\t'))
            cardstr[--clen] = '\0';

        char mnemonic[MAX_MNEMONIC] = {0};
        if (f3) {
            while (*f3 == ' ' || *f3 == '\t') f3++;
            strncpy(mnemonic, f3, sizeof(mnemonic) - 1);
            int mlen = strlen(mnemonic);
            while (mlen > 0 && (mnemonic[mlen-1] == ' ' || mnemonic[mlen-1] == '\t'
                                 || mnemonic[mlen-1] == '\n'))
                mnemonic[--mlen] = '\0';
        }

        StackEntry *e = &s->entries[s->count];
        e->position = pos;
        if (card_parse(cardstr, &e->card) != 0) continue;
        snprintf(e->mnemonic, sizeof(e->mnemonic), "%s", mnemonic);
        s->count++;
    }

    /* sort by position */
    for (int i = 0; i < s->count - 1; i++)
        for (int j = i + 1; j < s->count; j++)
            if (s->entries[j].position < s->entries[i].position) {
                StackEntry tmp = s->entries[i];
                s->entries[i] = s->entries[j];
                s->entries[j] = tmp;
            }
    return 0;
}

/* ── Minimal JSON getters (for progress restore) ─────────────────────────── */

static int json_int(const char *js, const char *key, int def)
{
    char search[128];
    snprintf(search, sizeof(search), "\"%s\":", key);
    const char *p = strstr(js, search);
    if (!p) return def;
    p += strlen(search);
    while (*p == ' ') p++;
    return atoi(p);
}

static void json_str(const char *js, const char *key, char *out, int outlen)
{
    char search[128];
    snprintf(search, sizeof(search), "\"%s\":\"", key);
    const char *p = strstr(js, search);
    if (!p) { if (outlen) out[0] = '\0'; return; }
    p += strlen(search);
    int i = 0;
    while (*p && *p != '"' && i < outlen - 1) out[i++] = *p++;
    out[i] = '\0';
}

static void json_int_arr(const char *js, const char *key, int *arr, int arrlen)
{
    char search[128];
    snprintf(search, sizeof(search), "\"%s\":[", key);
    const char *p = strstr(js, search);
    if (!p) return;
    p += strlen(search);
    for (int i = 0; i < arrlen; i++) {
        while (*p == ' ') p++;
        arr[i] = atoi(p);
        while (*p && *p != ',' && *p != ']') p++;
        if (*p == ',') p++; else break;
    }
}

/* ── Exported API ─────────────────────────────────────────────────────────── */

EMSCRIPTEN_KEEPALIVE
void md_init(void)
{
    memset(&g_app, 0, sizeof(g_app));
    g_app.settings.question_type = Q_POS_TO_CARD;
    g_app.settings.num_choices   = DEFAULT_CHOICES;
    g_app.settings.range_min     = 1;
    g_app.settings.range_max     = STACK_SIZE;
    g_app.settings.limit_mode    = LIMIT_NONE;
    g_app.settings.limit_value   = 10;
    g_app.settings.card_filter   = FILTER_ALL;
    g_app.settings.randomize     = 1;
    snprintf(g_app.progress_file, sizeof(g_app.progress_file),
             "/tmp/memdeck.dat");
    srand((unsigned)time(NULL));
}

/* Load a stack from TSV text; returns stack index or -1 on error */
EMSCRIPTEN_KEEPALIVE
int md_load_stack(const char *name, const char *tsv)
{
    if (g_app.stack_count >= MAX_STACKS) return -1;
    Stack *s = &g_app.stacks[g_app.stack_count];
    memset(s, 0, sizeof(*s));
    if (parse_stack_tsv(s, tsv) != 0 || s->count == 0) return -1;
    strncpy(s->name, name, sizeof(s->name) - 1);
    s->builtin = 1;
    return g_app.stack_count++;
}

EMSCRIPTEN_KEEPALIVE int md_stack_count(void) { return g_app.stack_count; }

EMSCRIPTEN_KEEPALIVE
const char *md_stack_name(int idx)
{
    if (idx < 0 || idx >= g_app.stack_count) return "";
    return g_app.stacks[idx].name;
}

EMSCRIPTEN_KEEPALIVE
int md_stack_size(int idx)
{
    if (idx < 0 || idx >= g_app.stack_count) return 0;
    return g_app.stacks[idx].count;
}

EMSCRIPTEN_KEEPALIVE int md_get_current_stack(void) { return g_app.current_stack; }

EMSCRIPTEN_KEEPALIVE
void md_set_current_stack(int idx)
{
    if (idx >= 0 && idx < g_app.stack_count)
        g_app.current_stack = idx;
}

EMSCRIPTEN_KEEPALIVE
const char *md_get_entry(int stack_idx, int entry_idx)
{
    int pos = 0;
    g_json[0] = '\0';
    if (entry_json(g_json, sizeof(g_json), &pos, &g_app, stack_idx, entry_idx) != 0)
        snprintf(g_json, sizeof(g_json), "{}");
    return g_json;
}

EMSCRIPTEN_KEEPALIVE
void md_set_settings(int q_type, int num_choices, int range_min, int range_max,
                     int limit_mode, int limit_value, int card_filter,
                     int show_mnemonic)
{
    g_app.settings.question_type = q_type;
    g_app.settings.num_choices   = num_choices;
    g_app.settings.range_min     = range_min;
    g_app.settings.range_max     = range_max;
    g_app.settings.limit_mode    = limit_mode;
    g_app.settings.limit_value   = limit_value;
    g_app.settings.card_filter   = card_filter;
    g_app.settings.show_mnemonic = show_mnemonic;
}

EMSCRIPTEN_KEEPALIVE
const char *md_get_settings(void)
{
    snprintf(g_json, sizeof(g_json),
        "{\"question_type\":%d,\"num_choices\":%d,\"range_min\":%d,"
        "\"range_max\":%d,\"limit_mode\":%d,\"limit_value\":%d,"
        "\"card_filter\":%d,\"show_mnemonic\":%d}",
        g_app.settings.question_type, g_app.settings.num_choices,
        g_app.settings.range_min, g_app.settings.range_max,
        g_app.settings.limit_mode, g_app.settings.limit_value,
        g_app.settings.card_filter, g_app.settings.show_mnemonic);
    return g_json;
}

EMSCRIPTEN_KEEPALIVE
void md_session_start(void) { session_init(&g_app); }

EMSCRIPTEN_KEEPALIVE
const char *md_session_question(void)
{
    Session *ss = &g_app.session;

    if (!ss->active) {
        snprintf(g_json, sizeof(g_json), "{\"active\":false}");
        return g_json;
    }

    int nc = g_app.settings.num_choices;
    if (nc > MAX_CHOICES) nc = MAX_CHOICES;

    /* Determine display type for choices based on resolved question type */
    int qtype = g_app.settings.question_type;
    if (qtype == Q_MIXED) {
        const char *qt = ss->question_text;
        if      (strstr(qt, "What position")) qtype = Q_CARD_TO_POS;
        else if (strstr(qt, "suit"))          qtype = Q_SUIT_DRILL;
        else if (strstr(qt, "value"))         qtype = Q_VALUE_DRILL;
        else                                  qtype = Q_POS_TO_CARD;
    }
    /* display_type: 0=card, 1=position, 2=suit, 3=value */
    int display_type;
    switch (qtype) {
    case Q_CARD_TO_POS:  display_type = 1; break;
    case Q_SUIT_DRILL:   display_type = 2; break;
    case Q_VALUE_DRILL:  display_type = 3; break;
    default:             display_type = 0; break;
    }

    /* Find which slot holds the canonical correct answer */
    int correct_slot = 0;
    for (int i = 0; i < nc; i++) {
        if (ss->choices[i] == ss->current_answer) { correct_slot = i; break; }
    }

    /* Build choices array */
    char choices[8192];
    int cp = 0;
    choices[cp++] = '[';
    for (int i = 0; i < nc; i++) {
        if (i > 0) choices[cp++] = ',';
        entry_json(choices, sizeof(choices), &cp, &g_app,
                   g_app.current_stack, ss->choices[i]);
    }
    choices[cp++] = ']';
    choices[cp] = '\0';

    /* Build current_entry JSON */
    char cur[1024];
    int curp = 0;
    entry_json(cur, sizeof(cur), &curp, &g_app,
               g_app.current_stack, ss->current_question);

    /* Escape question text */
    char qtext[512];
    json_escape(ss->question_text, qtext, sizeof(qtext));

    snprintf(g_json, sizeof(g_json),
        "{\"active\":true,\"text\":\"%s\","
        "\"display_type\":%d,\"num_choices\":%d,"
        "\"choices\":%s,\"correct_slot\":%d,"
        "\"answered\":%s,\"last_correct\":%s,\"selected\":%d,"
        "\"correct\":%d,\"incorrect\":%d,\"streak\":%d,"
        "\"best_streak\":%d,\"lives\":%d,\"questions_asked\":%d,"
        "\"current_entry\":%s}",
        qtext, display_type, nc,
        choices, correct_slot,
        ss->answered ? "true" : "false",
        ss->last_correct ? "true" : "false",
        ss->selected,
        ss->correct, ss->incorrect, ss->streak,
        ss->best_streak, ss->lives, ss->questions_asked,
        cur);
    return g_json;
}

EMSCRIPTEN_KEEPALIVE
void md_session_answer(int choice_idx) { session_check_answer(&g_app, choice_idx); }

EMSCRIPTEN_KEEPALIVE
void md_session_next(void)
{
    if (g_app.session.answered && !session_is_over(&g_app))
        session_generate_question(&g_app);
}

EMSCRIPTEN_KEEPALIVE
int md_session_is_over(void) { return session_is_over(&g_app); }

EMSCRIPTEN_KEEPALIVE
const char *md_session_stats(void)
{
    Session *ss = &g_app.session;
    time_t elapsed = time(NULL) - ss->start_time;
    snprintf(g_json, sizeof(g_json),
        "{\"correct\":%d,\"incorrect\":%d,\"streak\":%d,"
        "\"best_streak\":%d,\"questions_asked\":%d,\"elapsed_sec\":%d}",
        ss->correct, ss->incorrect, ss->streak,
        ss->best_streak, ss->questions_asked, (int)elapsed);
    return g_json;
}

/* Call when a session ends to update persistent progress counters */
EMSCRIPTEN_KEEPALIVE
void md_session_complete(void)
{
    Session  *ss = &g_app.session;
    Progress *p  = &g_app.progress;
    p->total_sessions++;
    if (ss->correct > p->best_score)
        p->best_score = ss->correct;
    const char *today = progress_today();
    if (p->last_date[0] == '\0') {
        p->current_streak = 1;
    } else if (strcmp(p->last_date, today) != 0) {
        p->current_streak++;
    }
    if (p->current_streak > p->best_streak)
        p->best_streak = p->current_streak;
    strncpy(p->last_date, today, sizeof(p->last_date) - 1);
}

EMSCRIPTEN_KEEPALIVE
const char *md_progress_dump(void)
{
    Progress *p = &g_app.progress;
    char errs[1024], cors[1024];
    int ep = 0, cp = 0;
    errs[ep++] = '['; cors[cp++] = '[';
    for (int i = 0; i < STACK_SIZE; i++) {
        if (i > 0) { errs[ep++] = ','; cors[cp++] = ','; }
        ep += snprintf(errs + ep, sizeof(errs) - ep - 2, "%d", p->card_errors[i]);
        cp += snprintf(cors + cp, sizeof(cors) - cp - 2, "%d", p->card_correct[i]);
    }
    errs[ep++] = ']'; errs[ep] = '\0';
    cors[cp++] = ']'; cors[cp] = '\0';

    snprintf(g_json, sizeof(g_json),
        "{\"total_sessions\":%d,\"total_correct\":%d,\"total_incorrect\":%d,"
        "\"best_score\":%d,\"current_streak\":%d,\"best_streak\":%d,"
        "\"last_date\":\"%s\",\"card_errors\":%s,\"card_correct\":%s}",
        p->total_sessions, p->total_correct, p->total_incorrect,
        p->best_score, p->current_streak, p->best_streak,
        p->last_date, errs, cors);
    return g_json;
}

EMSCRIPTEN_KEEPALIVE
void md_progress_load(const char *json)
{
    if (!json || !json[0]) return;
    Progress *p = &g_app.progress;
    p->total_sessions  = json_int(json, "total_sessions",  0);
    p->total_correct   = json_int(json, "total_correct",   0);
    p->total_incorrect = json_int(json, "total_incorrect", 0);
    p->best_score      = json_int(json, "best_score",      0);
    p->current_streak  = json_int(json, "current_streak",  0);
    p->best_streak     = json_int(json, "best_streak",     0);
    json_str(json, "last_date", p->last_date, sizeof(p->last_date));
    json_int_arr(json, "card_errors",  p->card_errors,  STACK_SIZE);
    json_int_arr(json, "card_correct", p->card_correct, STACK_SIZE);
}

EMSCRIPTEN_KEEPALIVE
void md_progress_reset(void) { progress_reset(&g_app); }

EMSCRIPTEN_KEEPALIVE
int md_filtered_count(void) { return g_app.session.filtered_count; }

EMSCRIPTEN_KEEPALIVE
const char *md_validate_stack(int idx)
{
    if (idx < 0 || idx >= g_app.stack_count) return "Invalid stack index";
    static char errbuf[256];
    if (stack_validate(&g_app.stacks[idx], errbuf, sizeof(errbuf)) != 0)
        return errbuf;
    return "";
}
