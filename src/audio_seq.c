#include "audio_seq.h"

#include <string.h>

typedef struct {
    uint64_t remainder;
    uint64_t denom;
    int sample_rate;
    int tempo_bpm;
    int steps_per_beat;
    int swing_pct;
} SeqTimingState;

static int clampi(int value, int lo, int hi)
{
    if (value < lo) return lo;
    if (value > hi) return hi;
    return value;
}

static void seq_timing_init(SeqTimingState *state, int sample_rate,
                            int tempo_bpm, int steps_per_beat, int swing_pct)
{
    memset(state, 0, sizeof(*state));
    state->sample_rate = sample_rate;
    state->tempo_bpm = tempo_bpm > 0 ? tempo_bpm : 120;
    state->steps_per_beat = steps_per_beat > 0 ? steps_per_beat : 4;
    state->swing_pct = clampi(swing_pct, 50, 75);
}

static int seq_timing_next(SeqTimingState *state, int absolute_step)
{
    uint64_t numer = 0;

    if (state->sample_rate <= 0 || state->tempo_bpm <= 0 || state->steps_per_beat <= 0)
        return 0;

    if (state->swing_pct == 50 || state->steps_per_beat < 2) {
        state->denom = (uint64_t)state->tempo_bpm * (uint64_t)state->steps_per_beat;
        numer = (uint64_t)state->sample_rate * 60ull;
    } else {
        int weight = (absolute_step & 1) ? (100 - state->swing_pct) : state->swing_pct;
        state->denom = (uint64_t)state->tempo_bpm * (uint64_t)state->steps_per_beat * 50ull;
        numer = (uint64_t)state->sample_rate * 60ull * (uint64_t)weight;
    }

    {
        uint64_t sum = numer + state->remainder;
        int samples = (int)(sum / state->denom);
        state->remainder = sum % state->denom;
        return samples;
    }
}

int seq_song_total_steps(const SeqSong *song)
{
    int total = 0;

    if (!song) return 0;
    for (int i = 0; i < song->arrangement_length; i++) {
        int pattern_index = song->arrangement[i];
        if (pattern_index < 0 || pattern_index >= song->pattern_count)
            return -1;
        total += song->patterns[pattern_index].length;
    }
    return total;
}

int seq_compile_timeline(const SeqSong *song, int sample_rate, SeqTimeline *timeline)
{
    int absolute_step = 0;
    int max_track_count = 0;
    int total_steps;
    SeqTimingState timing;

    if (!song || !timeline) return -1;

    total_steps = seq_song_total_steps(song);
    if (total_steps <= 0 || total_steps > SEQ_MAX_TIMELINE_STEPS)
        return -1;

    memset(timeline, 0, sizeof(*timeline));
    seq_timing_init(&timing, sample_rate, song->tempo_bpm, song->steps_per_beat, song->swing_pct);

    for (int i = 0; i < song->arrangement_length; i++) {
        int pattern_index = song->arrangement[i];
        const SeqPattern *pattern = &song->patterns[pattern_index];

        if (pattern->length < 0 || pattern->length > SEQ_MAX_STEPS ||
            pattern->track_count < 0 || pattern->track_count > SEQ_MAX_TRACKS)
            return -1;

        if (pattern->track_count > max_track_count)
            max_track_count = pattern->track_count;

        for (int step = 0; step < pattern->length; step++) {
            SeqTimelineStep *entry = &timeline->steps[absolute_step];
            entry->pattern_index = pattern_index;
            entry->pattern_step = step;
            entry->samples = seq_timing_next(&timing, absolute_step);
            timeline->total_samples += entry->samples;
            absolute_step++;
        }
    }

    timeline->total_steps = absolute_step;
    timeline->max_track_count = max_track_count;
    return 0;
}
