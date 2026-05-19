#ifndef MEMDECK_AUDIO_JAM_H
#define MEMDECK_AUDIO_JAM_H

#include <stdint.h>
#include "audio_seq.h"

/*
 * audio_jam — variation primitives for "infinite continuation" mode.
 *
 * Strategy: keep a const base SeqSong, copy it into a working SeqSong per
 * section, apply a handful of pseudo-random mutations, hand the result
 * back to the render path. The base is never modified, so successive
 * sections are deterministic given a seed.
 *
 * All variations are SeqSong-level (no ABC text manipulation) and use
 * Q-style integer math; the PRNG is xorshift64*. No floats.
 */

typedef struct {
    uint64_t state;       /* xorshift64* state */
    int iteration;        /* increments per audio_jam_vary_song call */
    int arrangement_offset; /* current scroll position into base arrangement */
} JamState;

void     audio_jam_init(JamState *jam, uint64_t seed);
uint32_t audio_jam_rand(JamState *jam);
int      audio_jam_rand_range(JamState *jam, int min_inclusive, int max_exclusive);

/* Copy `slots` arrangement entries from `base`, starting at `start_slot`
 * (modulo base->arrangement_length), into `out`. The rest of `out` is
 * a shallow copy of base. `slots` is clamped to SEQ_MAX_ARRANGEMENT. */
void audio_jam_slice_song(SeqSong *out, const SeqSong *base,
                          int start_slot, int slots);

/* Pick how many arrangement slots cover roughly `section_seconds`
 * of audio in `base`. Always returns at least 1, capped at the base
 * arrangement_length. */
int audio_jam_slots_for_section(const SeqSong *base, double section_seconds);

/* In-place mutate the SeqSong. Apply the four MVP strategies:
 *   - velocity humanization (always)
 *   - arrangement shuffle (60% probability per call)
 *   - drum fill on last bar (50%)
 *   - voice/pattern mute (30%)
 * Increments jam->iteration. */
void audio_jam_vary_song(SeqSong *song, JamState *jam);

#endif
