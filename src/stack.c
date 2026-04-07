#include "memdeck.h"

int stack_load(Stack *s, const char *path)
{
    FILE *f = fopen(path, "r");
    if (!f) return -1;

    char line[1024];
    s->count = 0;

    while (fgets(line, sizeof(line), f) && s->count < STACK_SIZE) {
        /* skip comments and empty lines */
        char *p = line;
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '#' || *p == '\n' || *p == '\0') continue;

        /* parse: position <tab> card [<tab> mnemonic] */
        int pos;
        char cardstr[16] = {0};
        char mnemonic[MAX_MNEMONIC] = {0};

        /* find tab-separated fields */
        char *field1 = strtok(p, "\t\n");
        char *field2 = strtok(NULL, "\t\n");
        char *field3 = strtok(NULL, "\n");

        if (!field1 || !field2) continue;

        pos = atoi(field1);
        if (pos < 1 || pos > 52) continue;

        strncpy(cardstr, field2, sizeof(cardstr) - 1);

        /* trim whitespace from card string */
        int len = strlen(cardstr);
        while (len > 0 && (cardstr[len-1] == ' ' || cardstr[len-1] == '\t'))
            cardstr[--len] = '\0';

        if (field3) {
            /* trim leading whitespace */
            while (*field3 == ' ' || *field3 == '\t') field3++;
            strncpy(mnemonic, field3, sizeof(mnemonic) - 1);
            len = strlen(mnemonic);
            while (len > 0 && (mnemonic[len-1] == ' ' || mnemonic[len-1] == '\t' || mnemonic[len-1] == '\n'))
                mnemonic[--len] = '\0';
        }

        StackEntry *e = &s->entries[s->count];
        e->position = pos;
        if (card_parse(cardstr, &e->card) != 0) continue;
        snprintf(e->mnemonic, sizeof(e->mnemonic), "%s", mnemonic);
        s->count++;
    }

    fclose(f);

    /* sort by position */
    for (int i = 0; i < s->count - 1; i++) {
        for (int j = i + 1; j < s->count; j++) {
            if (s->entries[j].position < s->entries[i].position) {
                StackEntry tmp = s->entries[i];
                s->entries[i] = s->entries[j];
                s->entries[j] = tmp;
            }
        }
    }

    /* extract name from filename */
    const char *base = strrchr(path, '/');
    base = base ? base + 1 : path;
    strncpy(s->filename, path, sizeof(s->filename) - 1);
    strncpy(s->name, base, sizeof(s->name) - 1);
    /* remove .tsv extension */
    char *dot = strrchr(s->name, '.');
    if (dot) *dot = '\0';
    /* capitalize first letter */
    if (s->name[0]) s->name[0] = toupper(s->name[0]);

    return 0;
}

int stack_save(const Stack *s, const char *path)
{
    FILE *f = fopen(path, "w");
    if (!f) return -1;

    fprintf(f, "# %s\n", s->name);
    fprintf(f, "# position\tcard\tmnemonic\n");

    for (int i = 0; i < s->count; i++) {
        char code[8];
        card_code(&s->entries[i].card, code, sizeof(code));
        if (s->entries[i].mnemonic[0]) {
            fprintf(f, "%d\t%s\t%s\n", s->entries[i].position, code, s->entries[i].mnemonic);
        } else {
            fprintf(f, "%d\t%s\n", s->entries[i].position, code);
        }
    }

    fclose(f);
    return 0;
}

int stack_validate(const Stack *s, char *errbuf, int errlen)
{
    if (s->count != STACK_SIZE) {
        snprintf(errbuf, errlen, "Stack has %d entries, expected %d", s->count, STACK_SIZE);
        return -1;
    }

    /* check unique positions */
    int pos_seen[53] = {0};
    for (int i = 0; i < s->count; i++) {
        int p = s->entries[i].position;
        if (p < 1 || p > 52) {
            snprintf(errbuf, errlen, "Invalid position %d at entry %d", p, i + 1);
            return -1;
        }
        if (pos_seen[p]) {
            snprintf(errbuf, errlen, "Duplicate position %d", p);
            return -1;
        }
        pos_seen[p] = 1;
    }

    /* check unique cards */
    int card_seen[14][4] = {{0}};
    for (int i = 0; i < s->count; i++) {
        int r = s->entries[i].card.rank;
        int su = s->entries[i].card.suit;
        if (r < 1 || r > 13 || su < 0 || su > 3) {
            snprintf(errbuf, errlen, "Invalid card at position %d", s->entries[i].position);
            return -1;
        }
        if (card_seen[r][su]) {
            char code[8];
            card_code(&s->entries[i].card, code, sizeof(code));
            snprintf(errbuf, errlen, "Duplicate card %s", code);
            return -1;
        }
        card_seen[r][su] = 1;
    }

    return 0;
}

void stack_discover(App *app)
{
    app->stack_count = 0;

    /* load from data dir (built-in stacks) */
    DIR *d = opendir(app->data_dir);
    if (d) {
        struct dirent *ent;
        while ((ent = readdir(d)) && app->stack_count < MAX_STACKS) {
            char *ext = strrchr(ent->d_name, '.');
            if (!ext || strcmp(ext, ".tsv") != 0) continue;

            char path[MAX_PATH + 512];
            snprintf(path, sizeof(path), "%s/%s", app->data_dir, ent->d_name);

            Stack *s = &app->stacks[app->stack_count];
            if (stack_load(s, path) == 0 && s->count > 0) {
                s->builtin = 1;
                app->stack_count++;
            }
        }
        closedir(d);
    }

    /* load from user dir (custom stacks) */
    char user_stacks[MAX_PATH + 64];
    snprintf(user_stacks, sizeof(user_stacks), "%s/stacks", app->user_dir);
    d = opendir(user_stacks);
    if (d) {
        struct dirent *ent;
        while ((ent = readdir(d)) && app->stack_count < MAX_STACKS) {
            char *ext = strrchr(ent->d_name, '.');
            if (!ext || strcmp(ext, ".tsv") != 0) continue;

            char path[MAX_PATH + 512];
            snprintf(path, sizeof(path), "%s/%s", user_stacks, ent->d_name);

            Stack *s = &app->stacks[app->stack_count];
            if (stack_load(s, path) == 0 && s->count > 0) {
                s->builtin = 0;
                app->stack_count++;
            }
        }
        closedir(d);
    }

    /* sort: built-in first, then alphabetical */
    for (int i = 0; i < app->stack_count - 1; i++) {
        for (int j = i + 1; j < app->stack_count; j++) {
            int swap = 0;
            if (app->stacks[j].builtin && !app->stacks[i].builtin) swap = 1;
            else if (app->stacks[j].builtin == app->stacks[i].builtin &&
                     strcmp(app->stacks[j].name, app->stacks[i].name) < 0) swap = 1;
            if (swap) {
                Stack tmp = app->stacks[i];
                app->stacks[i] = app->stacks[j];
                app->stacks[j] = tmp;
            }
        }
    }
}
