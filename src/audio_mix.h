#ifndef MEMDECK_AUDIO_MIX_H
#define MEMDECK_AUDIO_MIX_H

#include "audio_seq.h"

unsigned char *audio_mix_render_song(const SeqSong *song, int sample_rate, int *out_len);
unsigned char *audio_mix_render_timeline(const SeqSong *song, const SeqTimeline *timeline,
                                         int sample_rate, int *out_len);

#endif
