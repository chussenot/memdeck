#include "memdeck.h"
#include "audio_engine.h"
#include "audio_dsp.h"
#include "miniaudio_playback.h"
#include <stdlib.h>
#include <string.h>

/*
 * Native audio backend using miniaudio.
 *
 * SFX:   short one-shot tones played via a dedicated MaPlaybackHandle.
 * Music: looping background track rendered by audio_engine, played in a
 *        looping MaPlaybackHandle.  No external tool (aplay, afplay, etc.)
 *        is required — miniaudio loads the platform audio driver at runtime.
 */

#define SAMPLE_RATE 22050
#define SFX_AMP     96
#ifndef MEMDECK_AUDIO_PROFILE
#define MEMDECK_AUDIO_PROFILE 0
#endif

static DspProfile g_sound_profile;
static int g_last_loop_samples = 0;

/* ── Helpers ─────────────────────────────────────────────────────────────── */

static int gen_tone(unsigned char *buf, int max_samples, double freq, int duration_ms,
                    int amplitude, DspWaveform waveform)
{
    int n = dsp_samples_from_ms(SAMPLE_RATE, duration_ms);
    if (n > max_samples) n = max_samples;
    if (freq <= 0.0) {
        memset(buf, 128, (size_t)n);
        return n;
    }
    DspOscillator osc;
    dsp_osc_init(&osc, waveform, amplitude);
    dsp_osc_set_frequency(&osc, freq, SAMPLE_RATE);
    for (int i = 0; i < n; i++)
        buf[i] = (unsigned char)dsp_clamp_u8(128 + dsp_osc_next(&osc));
    return n;
}

/* ── SFX ─────────────────────────────────────────────────────────────────── */

/* One shared handle for all SFX (one-shot sounds, not looping). */
static MaPlaybackHandle *g_sfx_handle = NULL;

static void sfx_ensure_handle(void)
{
    if (!g_sfx_handle)
        g_sfx_handle = ma_pb_create();
}

static void sound_play(const unsigned char *data, int len)
{
    sfx_ensure_handle();
    if (!g_sfx_handle) return;
    ma_pb_start(g_sfx_handle, data, (size_t)len, SAMPLE_RATE, 0 /* no loop */);
}

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
    if (MEMDECK_AUDIO_PROFILE)
        dsp_profile_add_generation(&g_sound_profile, off, dsp_profile_now_ticks() - t0);
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
    if (MEMDECK_AUDIO_PROFILE)
        dsp_profile_add_generation(&g_sound_profile, off, dsp_profile_now_ticks() - t0);
    sound_play(buf, off);
}

/* ── Background music ────────────────────────────────────────────────────── */

static MaPlaybackHandle *g_music_handle = NULL;
static int   music_source = 0;   /* 0=none, 1=abc, 2=builtin sequencer */
static char  music_track_title[128] = {0};
static char  sound_data_dir[MAX_PATH + 64] = {0};

int sound_music_source(void) { return music_source; }
const char *sound_music_title(void) { return music_track_title; }

void sound_set_data_dir(const char *dir)
{
    snprintf(sound_data_dir, sizeof(sound_data_dir), "%s", dir);
}

/* Generate PCM for one loop iteration using ABC files. */
static unsigned char *music_try_track(int *out_len, const char *music_dir,
                                      const char *prefix)
{
    char bass_path[MAX_PATH + 128];
    char arp_path[MAX_PATH + 128];
    char lead_path[MAX_PATH + 128];

    snprintf(bass_path, sizeof(bass_path), "%s/%s_bass.abc", music_dir, prefix);
    snprintf(arp_path,  sizeof(arp_path),  "%s/%s_arp.abc",  music_dir, prefix);
    snprintf(lead_path, sizeof(lead_path), "%s/%s_lead.abc", music_dir, prefix);

    const char *paths[3] = { bass_path, arp_path, lead_path };
    AbcMusic music;

    if (abc_load_voices(paths, 3, &music) == 0 && music.voice_count > 0) {
        SeqSong song;
        snprintf(music_track_title, sizeof(music_track_title), "%.127s", music.title);
        if (abc_build_seq_song(&music, &song) != 0) return NULL;
        return audio_engine_render_song(&song, SAMPLE_RATE, out_len, NULL);
    }
    return NULL;
}

static unsigned char *music_load_abc(int *out_len, const char *data_dir)
{
    char music_dir[MAX_PATH + 96];
    snprintf(music_dir, sizeof(music_dir), "%s/../music", data_dir);

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

    if (count > 0) {
        int pick = rand() % count;
        unsigned char *buf = music_try_track(out_len, music_dir, prefixes[available[pick]]);
        if (buf) return buf;
    }

    /* Fallback: combined menu.abc */
    char combined[MAX_PATH + 128];
    snprintf(combined, sizeof(combined), "%s/menu.abc", music_dir);
    {
        AbcMusic music;
        if (abc_load(combined, &music) == 0 && music.voice_count > 0) {
            snprintf(music_track_title, sizeof(music_track_title), "%.127s", music.title);
            return audio_engine_render_abc_file(combined, SAMPLE_RATE, out_len, NULL);
        }
    }

    music_track_title[0] = '\0';
    return NULL;
}

static unsigned char *music_generate_loop(int *out_len)
{
    AudioRenderStats stats;
    uint64_t t0 = dsp_profile_now_ticks();
    unsigned char *buf = audio_engine_render_builtin_menu(SAMPLE_RATE, out_len, &stats);
    snprintf(music_track_title, sizeof(music_track_title), "%s",
             buf ? "MemDeck Built-in Retro Sequencer" : "");
    if (buf && out_len && MEMDECK_AUDIO_PROFILE)
        dsp_profile_add_generation(&g_sound_profile, *out_len, dsp_profile_now_ticks() - t0);
    return buf;
}

void sound_music_start(void)
{
    /* Don't restart if already playing. */
    if (g_music_handle && ma_pb_is_active(g_music_handle)) return;

    sound_music_stop();

    int loop_len = 0;
    unsigned char *loop_buf = NULL;

    if (sound_data_dir[0])
        loop_buf = music_load_abc(&loop_len, sound_data_dir);

    if (loop_buf) {
        music_source = 1;
    } else {
        loop_buf = music_generate_loop(&loop_len);
        if (loop_buf) music_source = 2;
    }

    if (!loop_buf) return;
    g_last_loop_samples = loop_len;

    if (!g_music_handle)
        g_music_handle = ma_pb_create();

    if (g_music_handle)
        ma_pb_start(g_music_handle, loop_buf, (size_t)loop_len, SAMPLE_RATE, 1 /* loop */);

    audio_engine_free_buffer(loop_buf);
}

void sound_music_stop(void)
{
    if (g_music_handle) {
        ma_pb_stop(g_music_handle);
        ma_pb_destroy(g_music_handle);
        g_music_handle = NULL;
    }
    music_source = 0;
}

/* ── Profiling ────────────────────────────────────────────────────────────── */

void sound_profile_reset(void)
{
    dsp_profile_reset(&g_sound_profile);
}

int sound_profile_snapshot(SoundProfile *out)
{
    if (!out) return -1;
    out->generated_samples     = g_sound_profile.generated_samples;
    out->generation_calls      = g_sound_profile.generation_calls;
    out->generation_ns         = dsp_profile_ticks_to_ns(g_sound_profile.generation_ticks);
    out->estimated_latency_ms  =
        (unsigned long long)((g_last_loop_samples * 1000ull) / SAMPLE_RATE);
    out->underruns             = g_sound_profile.underruns;
    return 0;
}
