#include "memdeck.h"
#include <signal.h>
#include <sys/wait.h>
#include <math.h>

/*
 * Chiptune sound engine.
 *
 * SFX:  short one-shot sounds (success/fail) forked and forgotten.
 * Music: looping background track with parent-owned pipe for reliable stop.
 *
 * All audio is generated as unsigned 8-bit PCM square waves and piped to aplay.
 */

#define SAMPLE_RATE 22050
#define SFX_AMP     96

/* ─── Helpers ─────────────────────────────────────────────────── */

/*
 * Reap finished SFX children only.
 * We must NOT use waitpid(-1) because that could accidentally reap
 * the music child processes (aplay_pid / writer_pid).
 */
static pid_t sfx_pids[16];
static int sfx_count = 0;

static void sfx_reap(void)
{
    int j = 0;
    for (int i = 0; i < sfx_count; i++) {
        if (waitpid(sfx_pids[i], NULL, WNOHANG) == 0) {
            /* still running, keep it */
            sfx_pids[j++] = sfx_pids[i];
        }
        /* else: reaped or error, drop it */
    }
    sfx_count = j;
}

static void sfx_track(pid_t pid)
{
    if (sfx_count < 16)
        sfx_pids[sfx_count++] = pid;
}

/* Generate a square wave tone. Returns number of samples written. */
static int gen_square(unsigned char *buf, int max_samples,
                      double freq, int duration_ms, int amplitude)
{
    int n = (SAMPLE_RATE * duration_ms) / 1000;
    if (n > max_samples) n = max_samples;
    if (freq <= 0.0) {
        memset(buf, 128, n);
        return n;
    }
    double half_period = SAMPLE_RATE / (2.0 * freq);
    for (int i = 0; i < n; i++) {
        int phase = (int)(i / half_period);
        buf[i] = (phase % 2 == 0)
            ? (unsigned char)(128 + amplitude)
            : (unsigned char)(128 - amplitude);
    }
    return n;
}

/* Fork + pipe PCM data to aplay for one-shot SFX. Returns immediately. */
static void sound_play(const unsigned char *data, int len)
{
    sfx_reap();
    pid_t pid = fork();
    if (pid < 0) return;
    if (pid > 0) { sfx_track(pid); return; }

    /* Child: pipe data to aplay and exit */
    int pipefd[2];
    if (pipe(pipefd) < 0) _exit(1);
    pid_t p2 = fork();
    if (p2 < 0) _exit(1);

    if (p2 == 0) {
        close(pipefd[1]);
        dup2(pipefd[0], STDIN_FILENO);
        close(pipefd[0]);
        int devnull = open("/dev/null", O_WRONLY);
        if (devnull >= 0) { dup2(devnull, STDERR_FILENO); close(devnull); }
        execlp("aplay", "aplay", "-q", "-f", "U8", "-r", "22050",
               "-c", "1", "-t", "raw", "--", (char *)NULL);
        _exit(0);
    }
    close(pipefd[0]);
    (void)!write(pipefd[1], data, len);
    close(pipefd[1]);
    waitpid(p2, NULL, 0);
    _exit(0);
}

/* ─── SFX ─────────────────────────────────────────────────────── */

void sound_success(void)
{
    static const double notes[] = { 523.25, 659.25, 783.99, 1046.50 };
    int dur = 60;
    int total = (SAMPLE_RATE * dur / 1000) * 4;
    unsigned char *buf = malloc(total);
    if (!buf) return;
    int off = 0;
    for (int i = 0; i < 4; i++)
        off += gen_square(buf + off, total - off, notes[i], dur, SFX_AMP);
    sound_play(buf, off);
    free(buf);
}

void sound_fail(void)
{
    static const double notes[] = { 392.00, 261.63 };
    int dur = 100;
    int total = (SAMPLE_RATE * dur / 1000) * 2;
    unsigned char *buf = malloc(total);
    if (!buf) return;
    int off = 0;
    for (int i = 0; i < 2; i++)
        off += gen_square(buf + off, total - off, notes[i], dur, SFX_AMP);
    sound_play(buf, off);
    free(buf);
}

/* ─── Background music ────────────────────────────────────────── */

/*
 * Architecture: the PARENT owns all resources for reliable cleanup.
 *
 *   Parent
 *     ├─ creates pipe
 *     ├─ forks aplay_pid  (reads pipe[0], execs aplay)
 *     ├─ forks writer_pid (writes loop PCM to pipe[1])
 *     └─ keeps pipe[1] open as music_fd
 *
 *   To stop:
 *     1. Kill writer  → no more data produced
 *     2. Close music_fd → pipe has no writers → aplay reads EOF → exits
 *     3. Kill aplay    → immediate stop (don't wait for buffer drain)
 *     4. Waitpid both  → clean reap, no zombies
 */

static pid_t aplay_pid  = 0;
static pid_t writer_pid = 0;
static int   music_fd   = -1;   /* parent's copy of pipe write-end */

/*
 * Dark synth chiptune in D minor, ~120 BPM.
 *
 * 3 channels mixed together:
 *   Bass  — low square wave, whole/half notes (amplitude 45)
 *   Arp   — mid-range pulsing 16th-note arpeggios, staccato (amplitude 28)
 *   Lead  — sparse haunting melody, high register (amplitude 40)
 *
 * 4-bar loop, 64 sixteenth notes total.
 * At 120 BPM: 16th note = 125ms = 2756 samples.
 */

/* Note frequencies (0 = rest) */
#define REST 0.0

#define A1   55.00
#define Bb1  58.27
#define D2   73.42
#define E2   82.41
#define F2   87.31
#define G2   98.00
#define A2  110.00

#define A3  220.00
#define Bb3 233.08
#define C4  261.63

#define D4  293.66
#define E4  329.63
#define F4  349.23
#define G4  392.00
#define A4  440.00
#define Bb4 466.16

#define C5  523.25
#define D5  587.33
#define E5  659.25
#define F5  698.46
#define A5  880.00
#define Bb5 932.33

#define STEPS 64
#define BPM   120
#define STEP_MS (60000 / BPM / 4)
#define STEP_SAMPLES (SAMPLE_RATE * STEP_MS / 1000)

#define NOTE_MS  (STEP_MS * 3 / 4)
#define GAP_MS   (STEP_MS - NOTE_MS)

#define BASS_AMP  45
#define ARP_AMP   28
#define LEAD_AMP  40

static const double bass_notes[STEPS] = {
    D2, D2, D2, D2,  D2, D2, D2, D2,  D2, D2, D2, D2,  D2, D2, D2, D2,
    Bb1,Bb1,Bb1,Bb1, Bb1,Bb1,Bb1,Bb1, A1, A1, A1, A1,  A1, A1, A1, A1,
    A1, A1, A1, A1,  A1, A1, A1, A1,  A1, A1, A1, A1,  A1, A1, A1, A1,
    A1, A1, A1, A1,  A1, A1, A1, A1,  Bb1,Bb1,Bb1,Bb1, D2, D2, D2, D2,
};

static const double arp_notes[STEPS] = {
    D4, F4, A4, D5,  D4, F4, A4, D5,  D4, F4, A4, D5,  D4, F4, A4, D5,
    Bb3,D4, F4, Bb4, Bb3,D4, F4, Bb4, A3, C4, E4, A4,  A3, C4, E4, A4,
    A3, C4, E4, A4,  A3, C4, E4, A4,  A3, C4, E4, A4,  A3, C4, E4, A4,
    A3, C4, E4, A4,  A3, C4, E4, A4,  Bb3,D4, F4, Bb4, D4, F4, A4, D5,
};

static const double lead_notes[STEPS] = {
    REST,REST,REST,REST, REST,REST,REST,REST, D5, D5, REST,REST, F5, E5, D5, D5,
    REST,REST,REST,REST, Bb4,Bb4,REST,REST,  C5, C5, REST,REST, REST,REST,REST,REST,
    REST,REST,REST,REST, A4, REST,C5, REST,  E5, E5, D5, D5,   REST,REST,REST,REST,
    REST,REST,REST,REST, REST,REST,REST,REST, A4, A4, Bb4,A4,   REST,REST,REST,REST,
};

static unsigned char *music_generate_loop(int *out_len)
{
    int loop_samples = STEPS * STEP_SAMPLES;
    unsigned char *buf = malloc(loop_samples);
    if (!buf) return NULL;

    memset(buf, 128, loop_samples);
    int note_samples = (SAMPLE_RATE * NOTE_MS) / 1000;

    for (int step = 0; step < STEPS; step++) {
        int base = step * STEP_SAMPLES;
        double bf = bass_notes[step];
        double af = arp_notes[step];
        double lf = lead_notes[step];

        double b_hp = (bf > 0) ? SAMPLE_RATE / (2.0 * bf) : 0;
        double a_hp = (af > 0) ? SAMPLE_RATE / (2.0 * af) : 0;
        double l_hp = (lf > 0) ? SAMPLE_RATE / (2.0 * lf) : 0;
        int lead_samples = STEP_SAMPLES * 9 / 10;

        for (int i = 0; i < STEP_SAMPLES; i++) {
            int val = 128;
            if (bf > 0 && b_hp > 0) {
                int phase = (int)(i / b_hp);
                val += (phase % 2 == 0) ? BASS_AMP : -BASS_AMP;
            }
            if (af > 0 && a_hp > 0 && i < note_samples) {
                int phase = (int)(i / a_hp);
                val += (phase % 2 == 0) ? ARP_AMP : -ARP_AMP;
            }
            if (lf > 0 && l_hp > 0 && i < lead_samples) {
                int phase = (int)(i / l_hp);
                val += (phase % 2 == 0) ? LEAD_AMP : -LEAD_AMP;
            }
            if (val < 0) val = 0;
            if (val > 255) val = 255;
            buf[base + i] = (unsigned char)val;
        }
    }

    *out_len = loop_samples;
    return buf;
}

void sound_music_start(void)
{
    /* Don't start if already playing */
    if (writer_pid > 0 && kill(writer_pid, 0) == 0) return;
    if (aplay_pid > 0 && kill(aplay_pid, 0) == 0) return;

    /* Clean slate */
    sound_music_stop();

    int loop_len = 0;
    unsigned char *loop_buf = music_generate_loop(&loop_len);
    if (!loop_buf) return;

    /* Parent creates the pipe */
    int pipefd[2];
    if (pipe(pipefd) < 0) { free(loop_buf); return; }

    /* Fork aplay: reads from pipe[0] */
    aplay_pid = fork();
    if (aplay_pid < 0) {
        close(pipefd[0]); close(pipefd[1]);
        free(loop_buf);
        return;
    }
    if (aplay_pid == 0) {
        free(loop_buf);
        close(pipefd[1]);           /* child doesn't write */
        dup2(pipefd[0], STDIN_FILENO);
        close(pipefd[0]);
        int devnull = open("/dev/null", O_WRONLY);
        if (devnull >= 0) { dup2(devnull, STDERR_FILENO); close(devnull); }
        execlp("aplay", "aplay", "-q", "-f", "U8", "-r", "22050",
               "-c", "1", "-t", "raw", "--", (char *)NULL);
        _exit(1);
    }

    /* Parent no longer needs read end */
    close(pipefd[0]);

    /* Fork writer: writes loop PCM to pipe[1] forever */
    writer_pid = fork();
    if (writer_pid < 0) {
        close(pipefd[1]);
        kill(aplay_pid, SIGKILL);
        waitpid(aplay_pid, NULL, 0);
        aplay_pid = 0;
        free(loop_buf);
        return;
    }
    if (writer_pid == 0) {
        /* Writer child: loop forever writing PCM data */
        close(music_fd);  /* don't inherit parent's extra copy (not set yet, harmless) */
        for (;;) {
            ssize_t w = write(pipefd[1], loop_buf, loop_len);
            if (w <= 0) break;   /* pipe broken → aplay died → exit */
        }
        close(pipefd[1]);
        free(loop_buf);
        _exit(0);
    }

    /* Parent keeps write-end so we can close it to trigger EOF on aplay */
    music_fd = pipefd[1];
    free(loop_buf);
}

void sound_music_stop(void)
{
    /* 1. Kill writer first → stops producing data */
    if (writer_pid > 0) {
        kill(writer_pid, SIGKILL);
        waitpid(writer_pid, NULL, 0);
        writer_pid = 0;
    }

    /* 2. Close parent's pipe FD → aplay sees EOF (no more writers) */
    if (music_fd >= 0) {
        close(music_fd);
        music_fd = -1;
    }

    /* 3. Kill aplay → immediate silence, don't drain buffer */
    if (aplay_pid > 0) {
        kill(aplay_pid, SIGKILL);
        waitpid(aplay_pid, NULL, 0);
        aplay_pid = 0;
    }
}
