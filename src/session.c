#include "memdeck.h"

void session_build_filter(App *app)
{
    Stack *st = &app->stacks[app->current_stack];
    PracticeSettings *ps = &app->settings;
    Session *ss = &app->session;

    ss->filtered_count = 0;
    for (int i = 0; i < st->count; i++) {
        int pos = st->entries[i].position;
        if (pos < ps->range_min || pos > ps->range_max) continue;
        if (!card_matches_filter(&st->entries[i].card, ps->card_filter)) continue;
        ss->filtered_indices[ss->filtered_count++] = i;
    }
}

void session_init(App *app)
{
    Session *ss = &app->session;
    memset(ss, 0, sizeof(*ss));
    ss->active = 1;
    ss->start_time = time(NULL);
    ss->lives = (app->settings.limit_mode == LIMIT_LIVES) ? app->settings.limit_value : 3;

    session_build_filter(app);

    if (ss->filtered_count == 0) {
        ss->active = 0;
        return;
    }

    srand(time(NULL));
    session_generate_question(app);
}

static int pick_random_index(Session *ss)
{
    return ss->filtered_indices[rand() % ss->filtered_count];
}

static int pick_different_index(Session *ss, int exclude)
{
    if (ss->filtered_count <= 1) return exclude;
    int idx;
    int attempts = 0;
    do {
        idx = ss->filtered_indices[rand() % ss->filtered_count];
        attempts++;
    } while (idx == exclude && attempts < 100);
    return idx;
}

void session_generate_question(App *app)
{
    Stack *st = &app->stacks[app->current_stack];
    PracticeSettings *ps = &app->settings;
    Session *ss = &app->session;

    if (ss->filtered_count == 0) { ss->active = 0; return; }

    int qtype = ps->question_type;
    if (qtype == Q_MIXED) {
        qtype = rand() % 6; /* 0-5 */
    }

    int qi = pick_random_index(ss);
    StackEntry *qe = &st->entries[qi];
    ss->current_question = qi;
    ss->answered = 0;
    ss->selected = 0;
    ss->input_len = 0;
    ss->input_buf[0] = '\0';

    char card_disp[16];
    card_display(&qe->card, card_disp, sizeof(card_disp));

    switch (qtype) {
    case Q_POS_TO_CARD:
        snprintf(ss->question_text, sizeof(ss->question_text),
                 "What card is at position %d?", qe->position);
        ss->current_answer = qi;
        break;

    case Q_CARD_TO_POS:
        snprintf(ss->question_text, sizeof(ss->question_text),
                 "What position is %s?", card_disp);
        ss->current_answer = qi;
        break;

    case Q_NEXT_CARD:
        if (qi > 0) {
            StackEntry *prev = &st->entries[qi - 1];
            char pd[16];
            card_display(&prev->card, pd, sizeof(pd));
            snprintf(ss->question_text, sizeof(ss->question_text),
                     "What comes after %s (#%d)?", pd, prev->position);
        } else {
            snprintf(ss->question_text, sizeof(ss->question_text),
                     "What is the first card in the stack?");
        }
        ss->current_answer = qi;
        break;

    case Q_PREV_CARD:
        if (qi < st->count - 1) {
            StackEntry *next = &st->entries[qi + 1];
            char nd[16];
            card_display(&next->card, nd, sizeof(nd));
            snprintf(ss->question_text, sizeof(ss->question_text),
                     "What comes before %s (#%d)?", nd, next->position);
        } else {
            snprintf(ss->question_text, sizeof(ss->question_text),
                     "What is the last card in the stack?");
        }
        ss->current_answer = qi;
        break;

    case Q_SUIT_DRILL:
        snprintf(ss->question_text, sizeof(ss->question_text),
                 "What suit is at position %d?", qe->position);
        ss->current_answer = qi;
        break;

    case Q_VALUE_DRILL:
        snprintf(ss->question_text, sizeof(ss->question_text),
                 "What value is at position %d?", qe->position);
        ss->current_answer = qi;
        break;

    default:
        snprintf(ss->question_text, sizeof(ss->question_text),
                 "What card is at position %d?", qe->position);
        ss->current_answer = qi;
        break;
    }

    /* generate MCQ choices */
    int nc = ps->num_choices;
    if (nc > MAX_CHOICES) nc = MAX_CHOICES;
    if (nc > ss->filtered_count) nc = ss->filtered_count;

    /* place correct answer at random position */
    int correct_slot = rand() % nc;
    for (int i = 0; i < nc; i++) {
        if (i == correct_slot) {
            ss->choices[i] = qi;
        } else {
            int ci;
            int unique;
            int attempts = 0;
            do {
                ci = pick_different_index(ss, qi);
                unique = 1;
                for (int j = 0; j < i; j++) {
                    if (ss->choices[j] == ci) { unique = 0; break; }
                }
                if (ci == qi) unique = 0;
                attempts++;
            } while (!unique && attempts < 200);
            ss->choices[i] = ci;
        }
    }
}

void session_check_answer(App *app, int choice)
{
    Session *ss = &app->session;
    Stack *st = &app->stacks[app->current_stack];

    if (ss->answered) return;
    ss->answered = 1;
    ss->selected = choice;

    int correct_idx = ss->current_answer;
    int chosen_idx = ss->choices[choice];

    int qtype = app->settings.question_type;
    if (qtype == Q_MIXED) {
        /* determine actual type from question text */
        if (strstr(ss->question_text, "What card is at"))
            qtype = Q_POS_TO_CARD;
        else if (strstr(ss->question_text, "What position"))
            qtype = Q_CARD_TO_POS;
        else if (strstr(ss->question_text, "after"))
            qtype = Q_NEXT_CARD;
        else if (strstr(ss->question_text, "before"))
            qtype = Q_PREV_CARD;
        else if (strstr(ss->question_text, "suit"))
            qtype = Q_SUIT_DRILL;
        else if (strstr(ss->question_text, "value"))
            qtype = Q_VALUE_DRILL;
    }

    int is_correct = 0;
    switch (qtype) {
    case Q_SUIT_DRILL:
        is_correct = (st->entries[chosen_idx].card.suit == st->entries[correct_idx].card.suit);
        break;
    case Q_VALUE_DRILL:
        is_correct = (st->entries[chosen_idx].card.rank == st->entries[correct_idx].card.rank);
        break;
    default:
        is_correct = (chosen_idx == correct_idx);
        break;
    }

    ss->last_correct = is_correct;

    if (is_correct) {
        ss->correct++;
        ss->streak++;
        if (ss->streak > ss->best_streak)
            ss->best_streak = ss->streak;
        progress_update(app, 1, correct_idx);
    } else {
        ss->incorrect++;
        ss->streak = 0;
        if (app->settings.limit_mode == LIMIT_LIVES)
            ss->lives--;
        progress_update(app, 0, correct_idx);
    }

    ss->questions_asked++;
}

int session_is_over(App *app)
{
    Session *ss = &app->session;
    PracticeSettings *ps = &app->settings;

    if (!ss->active) return 1;

    switch (ps->limit_mode) {
    case LIMIT_TIME: {
        time_t elapsed = time(NULL) - ss->start_time;
        return elapsed >= ps->limit_value;
    }
    case LIMIT_QUESTIONS:
        return ss->questions_asked >= ps->limit_value;
    case LIMIT_LIVES:
        return ss->lives <= 0;
    default:
        return 0;
    }
}
