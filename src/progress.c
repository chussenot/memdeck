#include "memdeck.h"

const char *progress_today(void)
{
    static char buf[16];
    time_t now = time(NULL);
    struct tm *t = localtime(&now);
    strftime(buf, sizeof(buf), "%Y-%m-%d", t);
    return buf;
}

void progress_load(App *app)
{
    memset(&app->progress, 0, sizeof(app->progress));

    FILE *f = fopen(app->progress_file, "r");
    if (!f) return;

    char line[256];
    while (fgets(line, sizeof(line), f)) {
        char key[64];
        char val[192];
        if (sscanf(line, "%63[^=]=%191[^\n]", key, val) != 2) continue;

        if (strcmp(key, "total_sessions") == 0) app->progress.total_sessions = atoi(val);
        else if (strcmp(key, "total_correct") == 0) app->progress.total_correct = atoi(val);
        else if (strcmp(key, "total_incorrect") == 0) app->progress.total_incorrect = atoi(val);
        else if (strcmp(key, "best_score") == 0) app->progress.best_score = atoi(val);
        else if (strcmp(key, "current_streak") == 0) app->progress.current_streak = atoi(val);
        else if (strcmp(key, "best_streak") == 0) app->progress.best_streak = atoi(val);
        else if (strcmp(key, "last_date") == 0) strncpy(app->progress.last_date, val, 15);
        else if (strncmp(key, "err_", 4) == 0) {
            int idx = atoi(key + 4);
            if (idx >= 0 && idx < STACK_SIZE) app->progress.card_errors[idx] = atoi(val);
        }
        else if (strncmp(key, "cor_", 4) == 0) {
            int idx = atoi(key + 4);
            if (idx >= 0 && idx < STACK_SIZE) app->progress.card_correct[idx] = atoi(val);
        }
    }

    fclose(f);
}

void progress_save(const App *app)
{
    /* ensure directory exists */
    char dir[MAX_PATH];
    strncpy(dir, app->progress_file, sizeof(dir) - 1);
    char *slash = strrchr(dir, '/');
    if (slash) {
        *slash = '\0';
        /* mkdir -p equivalent (single level) */
        mkdir(dir, 0755);
    }

    FILE *f = fopen(app->progress_file, "w");
    if (!f) return;

    fprintf(f, "total_sessions=%d\n", app->progress.total_sessions);
    fprintf(f, "total_correct=%d\n", app->progress.total_correct);
    fprintf(f, "total_incorrect=%d\n", app->progress.total_incorrect);
    fprintf(f, "best_score=%d\n", app->progress.best_score);
    fprintf(f, "current_streak=%d\n", app->progress.current_streak);
    fprintf(f, "best_streak=%d\n", app->progress.best_streak);
    fprintf(f, "last_date=%s\n", app->progress.last_date);

    for (int i = 0; i < STACK_SIZE; i++) {
        if (app->progress.card_errors[i])
            fprintf(f, "err_%d=%d\n", i, app->progress.card_errors[i]);
        if (app->progress.card_correct[i])
            fprintf(f, "cor_%d=%d\n", i, app->progress.card_correct[i]);
    }

    fclose(f);
}

void progress_update(App *app, int correct, int position)
{
    if (correct) {
        app->progress.total_correct++;
        if (position >= 0 && position < STACK_SIZE)
            app->progress.card_correct[position]++;
    } else {
        app->progress.total_incorrect++;
        if (position >= 0 && position < STACK_SIZE)
            app->progress.card_errors[position]++;
    }
}

void progress_reset(App *app)
{
    memset(&app->progress, 0, sizeof(app->progress));
    progress_save(app);
}
