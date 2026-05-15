#ifndef MEMDECK_AUDIO_ENGINE_H
#define MEMDECK_AUDIO_ENGINE_H

/*
 * audio_engine.h — Public facade for the MemDeck audio render engine.
 *
 * All render functions return a heap-allocated U8 PCM buffer on success
 * and NULL on failure.  The caller owns the buffer and must release it
 * with audio_engine_free_buffer().  Passing NULL for out_stats is safe.
 *
 * Render path:
 *   audio_engine_render_builtin_menu() -> SeqSong -> mixer -> FX -> PCM
 *   audio_engine_render_abc_file()     -> ABC DSL -> SeqSong -> mixer -> FX -> PCM
 *   audio_engine_render_song()         -> SeqSong -> mixer -> FX -> PCM
 */

#include <stdint.h>

#include "audio_seq.h"

/* Diagnostics filled by every render call. */
typedef struct {
    unsigned long long sample_count;
    double duration_ms;
    int min_sample;
    int max_sample;
    int peak;
    unsigned long long clipping_count;
    uint64_t checksum;
    double render_time_ms;
} AudioRenderStats;

unsigned char *audio_engine_render_builtin_menu(int sample_rate, int *out_len,
                                                AudioRenderStats *out_stats);
unsigned char *audio_engine_render_abc_file(const char *path, int sample_rate,
                                            int *out_len, AudioRenderStats *out_stats);
unsigned char *audio_engine_render_song(const SeqSong *song, int sample_rate,
                                        int *out_len, AudioRenderStats *out_stats);
void audio_engine_free_buffer(unsigned char *buffer);

#endif
