#ifndef MEMDECK_AUDIO_JAM_H
#define MEMDECK_AUDIO_JAM_H

#include <stdint.h>
#include "audio_seq.h"
#include "memdeck.h"

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

/* Heuristic voice role used by the variation strategies. Filled in by
 * audio_jam_analyze_song; UNKNOWN means "treat as generic lead". */
typedef enum {
    JAM_VOICE_UNKNOWN = 0,
    JAM_VOICE_KICK,
    JAM_VOICE_SNARE_HAT,  /* any short noise hit that isn't the kick */
    JAM_VOICE_BASS,
    JAM_VOICE_LEAD,
    JAM_VOICE_PAD
} JamVoiceRole;

typedef struct {
    uint64_t state;       /* xorshift64* state */
    int iteration;        /* increments per audio_jam_vary_song call */
    int arrangement_offset; /* current scroll position into base arrangement */
    /* Per-voice role (0..SEQ_MAX_TRACKS-1). Defaults to UNKNOWN until
     * audio_jam_analyze_song runs. */
    JamVoiceRole roles[SEQ_MAX_TRACKS];
    /* For each track, a representative MIDI note pulled from the song
     * (first sounding step). Used when synthesising new hits (drum fills)
     * so we don't insert an alien pitch. */
    int reference_note[SEQ_MAX_TRACKS];
    /* Per-pattern density 0..100 — percentage of step-slots that have
     * a sounding note. Used to gate disruptive arrangement swaps. */
    int pattern_density[SEQ_MAX_PATTERNS];
} JamState;

void     audio_jam_init(JamState *jam, uint64_t seed);
uint32_t audio_jam_rand(JamState *jam);
int      audio_jam_rand_range(JamState *jam, int min_inclusive, int max_exclusive);

/* Populate jam->roles[], jam->reference_note[], and jam->pattern_density[]
 * by inspecting `music` (for instrument params) and `base` (for note
 * pitches and active-step counts). Safe to call multiple times. */
void audio_jam_analyze_song(JamState *jam, const AbcMusic *music, const SeqSong *base);

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

/* Opaque session handle: holds a parsed ABC song, its base SeqSong,
 * the jam PRNG state, and how many arrangement slots make up one
 * rendered section. Designed for FFI consumers (Rust GUI) that want
 * to stream successive sections without re-parsing the file. */
typedef struct AudioJamSession AudioJamSession;

/* Parse the ABC file, build the base SeqSong, init the PRNG, and
 * pick the section size. Returns NULL on parse/build failure or if
 * the song has no arrangement. */
AudioJamSession *audio_jam_session_open(const char *abc_path, uint64_t seed,
                                        double section_seconds);

/* Render the next section's PCM (caller frees via audio_engine_free_buffer).
 * Advances the session's arrangement scroll head + iteration counter.
 * Returns NULL on render failure. */
unsigned char *audio_jam_session_render_next(AudioJamSession *session,
                                             int sample_rate,
                                             int *out_pcm_len);

int audio_jam_session_iteration(const AudioJamSession *session);
int audio_jam_session_arrangement_offset(const AudioJamSession *session);
int audio_jam_session_slots_per_section(const AudioJamSession *session);

void audio_jam_session_close(AudioJamSession *session);

#endif
