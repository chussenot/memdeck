#include "memdeck.h"
#include "audio_fx.h"
#include "audio_mix.h"
#include "audio_song_builtin.h"

#include <stdlib.h>
#include <string.h>

static void render_stats_reset(AudioRenderStats *stats)
{
    if (!stats) return;
    memset(stats, 0, sizeof(*stats));
}

static uint64_t fnv1a64(const unsigned char *data, int len)
{
    uint64_t hash = 1469598103934665603ull;

    for (int i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= 1099511628211ull;
    }
    return hash;
}

static void render_stats_fill(AudioRenderStats *stats, const unsigned char *buffer,
                              int sample_count, int sample_rate,
                              clock_t start_ticks, clock_t end_ticks)
{
    AudioClipStats clip_stats;
    int minv = 255;
    int maxv = 0;
    int peak = 0;

    if (!stats) return;

    render_stats_reset(stats);
    if (!buffer || sample_count <= 0 || sample_rate <= 0)
        return;

    audio_fx_clip_stats_reset(&clip_stats);
    for (int i = 0; i < sample_count; i++) {
        int sample = (int)buffer[i];
        int offset = sample - 128;

        if (offset < 0) offset = -offset;
        if (sample < minv) minv = sample;
        if (sample > maxv) maxv = sample;
        if (offset > peak) peak = offset;
        audio_fx_clip_stats_push(&clip_stats, buffer[i]);
    }

    stats->sample_count = (unsigned long long)sample_count;
    stats->duration_ms = ((double)sample_count * 1000.0) / (double)sample_rate;
    stats->min_sample = minv;
    stats->max_sample = maxv;
    stats->peak = peak;
    stats->clipping_count = clip_stats.clipping_count;
    stats->checksum = fnv1a64(buffer, sample_count);
    stats->render_time_ms = ((double)(end_ticks - start_ticks) * 1000.0) / (double)CLOCKS_PER_SEC;
}

unsigned char *audio_engine_render_song(const SeqSong *song, int sample_rate, int *out_len,
                                        AudioRenderStats *out_stats)
{
    unsigned char *buffer;
    clock_t start_ticks;
    clock_t end_ticks;

    if (out_len) *out_len = 0;
    render_stats_reset(out_stats);
    if (!song || !out_len || sample_rate <= 0)
        return NULL;

    start_ticks = clock();
    buffer = audio_mix_render_song(song, sample_rate, out_len);
    end_ticks = clock();
    if (!buffer || *out_len <= 0) {
        if (out_len) *out_len = 0;
        free(buffer);
        return NULL;
    }

    render_stats_fill(out_stats, buffer, *out_len, sample_rate, start_ticks, end_ticks);
    return buffer;
}

unsigned char *audio_engine_render_builtin_menu(int sample_rate, int *out_len, AudioRenderStats *out_stats)
{
    const SeqSong *song = audio_builtin_menu_song();

    if (!song) {
        if (out_len) *out_len = 0;
        render_stats_reset(out_stats);
        return NULL;
    }
    return audio_engine_render_song(song, sample_rate, out_len, out_stats);
}

unsigned char *audio_engine_render_abc_file(const char *path, int sample_rate, int *out_len,
                                            AudioRenderStats *out_stats)
{
    AbcMusic music;
    SeqSong song;

    if (out_len) *out_len = 0;
    render_stats_reset(out_stats);
    if (!path || !out_len || sample_rate <= 0)
        return NULL;
    if (abc_load(path, &music) != 0)
        return NULL;
    if (abc_build_seq_song(&music, &song) != 0)
        return NULL;
    return audio_engine_render_song(&song, sample_rate, out_len, out_stats);
}

void audio_engine_free_buffer(unsigned char *buffer)
{
    free(buffer);
}
