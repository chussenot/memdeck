#include "memdeck.h"
#include <signal.h>
#include <sys/wait.h>
#include <math.h>

/*
 * Chiptune sound effects using square waves piped to aplay.
 * Sounds are played asynchronously (forked) so the UI is not blocked.
 */

#define SAMPLE_RATE 22050
#define AMPLITUDE   96

/* Reap zombie child processes from previous sound playback */
static void sound_reap_children(void)
{
    while (waitpid(-1, NULL, WNOHANG) > 0)
        ;
}

/* Generate a square wave tone into buf. Returns number of samples written. */
static int gen_square(unsigned char *buf, int max_samples,
                      double freq, int duration_ms, int amplitude)
{
    int n = (SAMPLE_RATE * duration_ms) / 1000;
    if (n > max_samples) n = max_samples;
    double half_period = SAMPLE_RATE / (2.0 * freq);

    for (int i = 0; i < n; i++) {
        int phase = (int)(i / half_period);
        buf[i] = (phase % 2 == 0)
            ? (unsigned char)(128 + amplitude)
            : (unsigned char)(128 - amplitude);
    }
    return n;
}

/* Play raw PCM data by forking and piping to aplay */
static void sound_play(const unsigned char *data, int len)
{
    sound_reap_children();

    pid_t pid = fork();
    if (pid < 0) return;

    if (pid == 0) {
        /* Child: pipe data to aplay */
        int pipefd[2];
        if (pipe(pipefd) < 0) _exit(1);

        pid_t p2 = fork();
        if (p2 < 0) _exit(1);

        if (p2 == 0) {
            /* Grandchild: exec aplay reading from pipe */
            close(pipefd[1]);
            dup2(pipefd[0], STDIN_FILENO);
            close(pipefd[0]);

            /* Silence stderr */
            int devnull = open("/dev/null", O_WRONLY);
            if (devnull >= 0) {
                dup2(devnull, STDERR_FILENO);
                close(devnull);
            }

            execlp("aplay", "aplay",
                   "-q",              /* quiet */
                   "-f", "U8",        /* unsigned 8-bit */
                   "-r", "22050",     /* sample rate */
                   "-c", "1",         /* mono */
                   "-t", "raw",       /* raw PCM */
                   "--",
                   (char *)NULL);
            /* If aplay not found, try paplay via raw pipe */
            _exit(0);
        }

        /* Child: write data to pipe and exit */
        close(pipefd[0]);
        (void)!write(pipefd[1], data, len);
        close(pipefd[1]);
        waitpid(p2, NULL, 0);
        _exit(0);
    }
    /* Parent returns immediately */
}

/*
 * Success sound: bright ascending arpeggio C5-E5-G5-C6
 * Classic chiptune "coin" / "correct" jingle
 */
void sound_success(void)
{
    /* C5=523, E5=659, G5=784, C6=1047 */
    static const double notes[] = { 523.25, 659.25, 783.99, 1046.50 };
    static const int dur_ms = 60;

    int total = (SAMPLE_RATE * dur_ms / 1000) * 4;
    unsigned char *buf = malloc(total);
    if (!buf) return;

    int offset = 0;
    for (int i = 0; i < 4; i++) {
        offset += gen_square(buf + offset, total - offset,
                             notes[i], dur_ms, AMPLITUDE);
    }

    sound_play(buf, offset);
    free(buf);
}

/*
 * Fail sound: descending two-note buzz G4-C4
 * Classic chiptune "wrong" / "damage" sound
 */
void sound_fail(void)
{
    /* G4=392, C4=262 */
    static const double notes[] = { 392.00, 261.63 };
    static const int dur_ms = 100;

    int total = (SAMPLE_RATE * dur_ms / 1000) * 2;
    unsigned char *buf = malloc(total);
    if (!buf) return;

    int offset = 0;
    for (int i = 0; i < 2; i++) {
        offset += gen_square(buf + offset, total - offset,
                             notes[i], dur_ms, AMPLITUDE);
    }

    sound_play(buf, offset);
    free(buf);
}
