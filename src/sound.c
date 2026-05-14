#include "memdeck.h"
#include "audio_dsp.h"
#include <signal.h>
#include <sys/wait.h>

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
#ifndef MEMDECK_AUDIO_PROFILE
#define MEMDECK_AUDIO_PROFILE 0
#endif

static DspProfile g_sound_profile;
static int g_last_loop_samples = 0;

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

static void profile_generation(int samples, uint64_t ticks)
{
    if (!MEMDECK_AUDIO_PROFILE) return;
    dsp_profile_add_generation(&g_sound_profile, samples, ticks);
}

static int gen_tone(unsigned char *buf, int max_samples, double freq, int duration_ms,
                    int amplitude, DspWaveform waveform)
{
    int n = dsp_samples_from_ms(SAMPLE_RATE, duration_ms);
    if (n > max_samples) n = max_samples;
    if (freq <= 0.0) {
        memset(buf, 128, n);
        return n;
    }
    DspOscillator osc;
    dsp_osc_init(&osc, waveform, amplitude);
    dsp_osc_set_frequency(&osc, freq, SAMPLE_RATE);
    for (int i = 0; i < n; i++) {
        buf[i] = (unsigned char)dsp_clamp_u8(128 + dsp_osc_next(&osc));
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
    int off = 0;
    while (off < len) {
        ssize_t w = write(pipefd[1], data + off, (size_t)(len - off));
        if (w <= 0) break;
        off += (int)w;
    }
    close(pipefd[1]);
    waitpid(p2, NULL, 0);
    _exit(0);
}

/* ─── SFX ─────────────────────────────────────────────────────── */

void sound_success(void)
{
    static const double notes[] = { 523.25, 659.25, 783.99, 1046.50 };
    enum { DUR_MS = 60 };
    enum { TOTAL = (SAMPLE_RATE * DUR_MS / 1000) * 4 };
    unsigned char buf[TOTAL];
    uint64_t t0 = dsp_profile_now_ticks();
    int off = 0;
    for (int i = 0; i < 4; i++)
        off += gen_tone(buf + off, TOTAL - off, notes[i], DUR_MS, SFX_AMP, DSP_WAVE_SQUARE);
    profile_generation(off, dsp_profile_now_ticks() - t0);
    sound_play(buf, off);
}

void sound_fail(void)
{
    static const double notes[] = { 392.00, 261.63 };
    enum { DUR_MS = 100 };
    enum { TOTAL = (SAMPLE_RATE * DUR_MS / 1000) * 2 };
    unsigned char buf[TOTAL];
    uint64_t t0 = dsp_profile_now_ticks();
    int off = 0;
    for (int i = 0; i < 2; i++)
        off += gen_tone(buf + off, TOTAL - off, notes[i], DUR_MS, SFX_AMP, DSP_WAVE_SQUARE);
    profile_generation(off, dsp_profile_now_ticks() - t0);
    sound_play(buf, off);
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
    int loop_samples = dsp_total_samples_for_steps(SAMPLE_RATE, STEP_MS, STEPS);
    unsigned char *buf = malloc(loop_samples);
    if (!buf) return NULL;

    memset(buf, 128, loop_samples);
    int note_samples = dsp_samples_from_ms(SAMPLE_RATE, NOTE_MS);
    DspSampleStepper stepper;
    dsp_stepper_init(&stepper, SAMPLE_RATE, STEP_MS);
    int base = 0;
    uint64_t t0 = dsp_profile_now_ticks();

    for (int step = 0; step < STEPS; step++) {
        int step_samples = dsp_stepper_next(&stepper);
        double bf = bass_notes[step];
        double af = arp_notes[step];
        double lf = lead_notes[step];
        int lead_samples = (step_samples * 9) / 10;
        DspOscillator bass, arp, lead;
        int bass_on = (bf > 0.0);
        int arp_on = (af > 0.0);
        int lead_on = (lf > 0.0);

        if (bass_on) {
            dsp_osc_init(&bass, DSP_WAVE_SQUARE, BASS_AMP);
            dsp_osc_set_frequency(&bass, bf, SAMPLE_RATE);
        }
        if (arp_on) {
            dsp_osc_init(&arp, DSP_WAVE_PULSE, ARP_AMP);
            dsp_osc_set_frequency(&arp, af, SAMPLE_RATE);
            dsp_osc_set_pulse_width_percent(&arp, 25);
        }
        if (lead_on) {
            dsp_osc_init(&lead, DSP_WAVE_SQUARE, LEAD_AMP);
            dsp_osc_set_frequency(&lead, lf, SAMPLE_RATE);
        }

        for (int i = 0; i < step_samples; i++) {
            int val = 128;
            if (bass_on) val += dsp_osc_next(&bass);
            if (arp_on && i < note_samples) val += dsp_osc_next(&arp);
            if (lead_on && i < lead_samples) val += dsp_osc_next(&lead);
            buf[base + i] = (unsigned char)dsp_clamp_u8(val);
        }
        base += step_samples;
    }

    profile_generation(loop_samples, dsp_profile_now_ticks() - t0);
    *out_len = loop_samples;
    return buf;
}

/*
 * Try to load music from ABC files. Discovers all available tracks
 * (menu_bass/arp/lead.abc, menu2_bass/arp/lead.abc, ...) and picks one
 * at random. Falls back to a combined menu.abc if no voice files found.
 */
static char music_track_title[128] = {0};

static unsigned char *music_try_track(int *out_len, const char *music_dir,
                                      const char *prefix)
{
    char bass_path[MAX_PATH + 128];
    char arp_path[MAX_PATH + 128];
    char lead_path[MAX_PATH + 128];

    snprintf(bass_path, sizeof(bass_path), "%s/%s_bass.abc", music_dir, prefix);
    snprintf(arp_path, sizeof(arp_path), "%s/%s_arp.abc", music_dir, prefix);
    snprintf(lead_path, sizeof(lead_path), "%s/%s_lead.abc", music_dir, prefix);

    const char *paths[3] = { bass_path, arp_path, lead_path };
    AbcMusic music;

    if (abc_load_voices(paths, 3, &music) == 0 && music.voice_count > 0) {
        snprintf(music_track_title, sizeof(music_track_title), "%.127s", music.title);
        return abc_generate_pcm(&music, out_len);
    }
    return NULL;
}

static unsigned char *music_load_abc(int *out_len, const char *data_dir)
{
    /* Derive music dir from data_dir (which points to data/stacks) */
    char music_dir[MAX_PATH + 96];
    snprintf(music_dir, sizeof(music_dir), "%s/../music", data_dir);

    /* Discover available tracks: menu, menu2, menu3, ... */
    static const char *prefixes[] = {
        "menu", "menu2", "menu3", "menu4", "menu5", "menu6", "menu7", "menu8"
    };
    int available[8];
    int count = 0;

    for (int i = 0; i < 8; i++) {
        char test[MAX_PATH + 128];
        snprintf(test, sizeof(test), "%s/%s_bass.abc", music_dir, prefixes[i]);
        if (access(test, R_OK) == 0)
            available[count++] = i;
    }

    /* Pick one at random */
    if (count > 0) {
        int pick = rand() % count;
        unsigned char *buf = music_try_track(out_len, music_dir, prefixes[available[pick]]);
        if (buf) return buf;
    }

    /* Fallback: try combined menu.abc */
    char combined[MAX_PATH + 128];
    snprintf(combined, sizeof(combined), "%s/menu.abc", music_dir);
    AbcMusic music;
    if (abc_load(combined, &music) == 0 && music.voice_count > 0) {
        snprintf(music_track_title, sizeof(music_track_title), "%.127s", music.title);
        return abc_generate_pcm(&music, out_len);
    }

    music_track_title[0] = '\0';
    return NULL;
}

const char *sound_music_title(void) { return music_track_title; }

/* Global: data_dir is set by sound_set_data_dir() before music starts */
static char sound_data_dir[MAX_PATH + 64] = {0};
static int  music_source = 0; /* 0=none, 1=abc, 2=hardcoded */

int sound_music_source(void) { return music_source; }

void sound_set_data_dir(const char *dir)
{
    snprintf(sound_data_dir, sizeof(sound_data_dir), "%s", dir);
}

void sound_music_start(void)
{
    /* Don't start if already playing */
    if (writer_pid > 0 && kill(writer_pid, 0) == 0) return;
    if (aplay_pid > 0 && kill(aplay_pid, 0) == 0) return;

    /* Clean slate */
    sound_music_stop();

    int loop_len = 0;
    unsigned char *loop_buf = NULL;

    /* Try ABC files first */
    if (sound_data_dir[0])
        loop_buf = music_load_abc(&loop_len, sound_data_dir);

    if (loop_buf) {
        music_source = 1; /* ABC */
    } else {
        /* Fallback to hardcoded music */
        loop_buf = music_generate_loop(&loop_len);
        if (loop_buf) music_source = 2; /* hardcoded */
    }

    if (!loop_buf) return;
    g_last_loop_samples = loop_len;

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
            int off = 0;
            while (off < loop_len) {
                ssize_t w = write(pipefd[1], loop_buf + off, (size_t)(loop_len - off));
                if (w <= 0) {
                    if (MEMDECK_AUDIO_PROFILE) {
                        dsp_profile_add_write(&g_sound_profile, (int)w, loop_len - off, 1);
                    }
                    close(pipefd[1]);
                    free(loop_buf);
                    _exit(0);
                }
                if (MEMDECK_AUDIO_PROFILE) {
                    dsp_profile_add_write(&g_sound_profile, (int)w, loop_len - off, 0);
                }
                off += (int)w;
            }
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

void sound_profile_reset(void)
{
    dsp_profile_reset(&g_sound_profile);
}

int sound_profile_snapshot(SoundProfile *out)
{
    if (!out) return -1;
    out->generated_samples = g_sound_profile.generated_samples;
    out->generation_calls = g_sound_profile.generation_calls;
    out->generation_ns = dsp_profile_ticks_to_ns(g_sound_profile.generation_ticks);
    out->estimated_latency_ms = (unsigned long long)((g_last_loop_samples * 1000ull) / SAMPLE_RATE);
    out->underruns = g_sound_profile.underruns;
    return 0;
}
