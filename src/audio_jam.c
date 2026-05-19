#include "audio_jam.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "memdeck.h"
#include "audio_engine.h"

/* xorshift64* — fine for musical variation; cheap, deterministic. */
static uint64_t xs64_next(uint64_t *s)
{
    uint64_t x = *s;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *s = x;
    return x * 0x2545F4914F6CDD1Dull;
}

void audio_jam_init(JamState *jam, uint64_t seed)
{
    if (!jam) return;
    jam->state = seed ? seed : 0xCAFEBABE12345678ull;
    jam->iteration = 0;
    jam->arrangement_offset = 0;
}

uint32_t audio_jam_rand(JamState *jam)
{
    if (!jam) return 0;
    return (uint32_t)(xs64_next(&jam->state) >> 32);
}

int audio_jam_rand_range(JamState *jam, int min_inclusive, int max_exclusive)
{
    if (!jam || max_exclusive <= min_inclusive) return min_inclusive;
    uint32_t span = (uint32_t)(max_exclusive - min_inclusive);
    return min_inclusive + (int)(audio_jam_rand(jam) % span);
}

int audio_jam_slots_for_section(const SeqSong *base, double section_seconds)
{
    if (!base || base->arrangement_length <= 0) return 0;
    if (section_seconds <= 0.0) return 1;

    int bpm = base->tempo_bpm > 0 ? base->tempo_bpm : 120;
    int spb = base->steps_per_beat > 0 ? base->steps_per_beat : 2;
    double steps_per_sec = ((double)bpm / 60.0) * (double)spb;
    int target_steps = (int)(section_seconds * steps_per_sec);
    if (target_steps < 1) target_steps = 1;

    int accumulated = 0;
    for (int i = 0; i < base->arrangement_length; i++) {
        int p = base->arrangement[i];
        if (p < 0 || p >= base->pattern_count) continue;
        int len = base->patterns[p].length;
        if (len < 1) len = 1;
        accumulated += len;
        if (accumulated >= target_steps) return i + 1;
    }
    return base->arrangement_length;
}

void audio_jam_slice_song(SeqSong *out, const SeqSong *base,
                          int start_slot, int slots)
{
    if (!out || !base) return;
    memcpy(out, base, sizeof(*out));
    if (base->arrangement_length <= 0 || slots <= 0) {
        out->arrangement_length = 0;
        return;
    }
    if (slots > SEQ_MAX_ARRANGEMENT) slots = SEQ_MAX_ARRANGEMENT;
    /* Normalise start within base arrangement length. */
    int base_len = base->arrangement_length;
    int s = ((start_slot % base_len) + base_len) % base_len;
    for (int i = 0; i < slots; i++) {
        out->arrangement[i] = base->arrangement[(s + i) % base_len];
    }
    out->arrangement_length = slots;
}

/* ------------ variation strategies ------------ */

static void perturb_velocities(SeqSong *song, JamState *jam)
{
    /* Jitter velocity by +-8 on every sounding step. Keeps the song's
     * rhythm intact but adds organic micro-variation. */
    for (int p = 0; p < song->pattern_count; p++) {
        SeqPattern *pat = &song->patterns[p];
        for (int t = 0; t < pat->track_count && t < SEQ_MAX_TRACKS; t++) {
            SeqTrack *track = &pat->tracks[t];
            for (int s = 0; s < pat->length && s < SEQ_MAX_STEPS; s++) {
                SeqStep *step = &track->steps[s];
                if (step->velocity == 0) continue;
                int delta = audio_jam_rand_range(jam, -8, 9);
                int v = (int)step->velocity + delta;
                if (v < 1) v = 1;
                if (v > 127) v = 127;
                step->velocity = (uint8_t)v;
            }
        }
    }
}

static void shuffle_arrangement(SeqSong *song, JamState *jam)
{
    if (song->arrangement_length < 2) return;
    /* Swap two random adjacent slots. Adjacent (not arbitrary) keeps
     * the macro-form recognisable — verses stay near verses. */
    int a = audio_jam_rand_range(jam, 0, song->arrangement_length - 1);
    int b = a + 1;
    uint8_t tmp = song->arrangement[a];
    song->arrangement[a] = song->arrangement[b];
    song->arrangement[b] = tmp;
}

static void drum_fill_last_bar(SeqSong *song, JamState *jam)
{
    if (song->pattern_count <= 0) return;
    int p_idx = audio_jam_rand_range(jam, 0, song->pattern_count);
    SeqPattern *pat = &song->patterns[p_idx];
    if (pat->track_count <= 0) return;
    /* Track 0 is the convention for the lead drum (kick) in every demo. */
    SeqTrack *track = &pat->tracks[0];
    int spb = song->steps_per_beat > 0 ? song->steps_per_beat : 4;
    int bar_steps = spb * 4; /* assume 4/4 — true for every showcase song */
    if (bar_steps > pat->length) bar_steps = pat->length;
    int start = pat->length - bar_steps;
    if (start < 0) start = 0;
    /* Fill: a hit on every step. Note value matches what the demos use
     * for kicks (middle C). */
    for (int s = start; s < pat->length; s++) {
        SeqStep *step = &track->steps[s];
        step->note = 60;
        step->velocity = 96;
        step->gate = 70;
        step->accent = (uint8_t)((s == start) ? 1 : 0);
        step->fx_trigger = step->accent;
    }
}

static void mute_voice_in_pattern(SeqSong *song, JamState *jam)
{
    if (song->pattern_count <= 0) return;
    int p_idx = audio_jam_rand_range(jam, 0, song->pattern_count);
    SeqPattern *pat = &song->patterns[p_idx];
    if (pat->track_count <= 0) return;
    int t_idx = audio_jam_rand_range(jam, 0, pat->track_count);
    SeqTrack *track = &pat->tracks[t_idx];
    for (int s = 0; s < pat->length; s++) {
        track->steps[s].velocity = 0;
        track->steps[s].gate = 0;
    }
}

void audio_jam_vary_song(SeqSong *song, JamState *jam)
{
    if (!song || !jam) return;
    jam->iteration++;

    perturb_velocities(song, jam);

    if (audio_jam_rand_range(jam, 0, 100) < 60)
        shuffle_arrangement(song, jam);

    if (audio_jam_rand_range(jam, 0, 100) < 50)
        drum_fill_last_bar(song, jam);

    if (audio_jam_rand_range(jam, 0, 100) < 30)
        mute_voice_in_pattern(song, jam);
}

/* ------------ session handle ------------ */

struct AudioJamSession {
    AbcMusic music;
    SeqSong base;
    JamState state;
    int slots_per_section;
};

AudioJamSession *audio_jam_session_open(const char *abc_path, uint64_t seed,
                                        double section_seconds)
{
    if (!abc_path) return NULL;
    AudioJamSession *s = calloc(1, sizeof(*s));
    if (!s) return NULL;
    if (abc_load(abc_path, &s->music) != 0) {
        free(s);
        return NULL;
    }
    if (abc_build_seq_song(&s->music, &s->base) != 0) {
        free(s);
        return NULL;
    }
    if (s->base.arrangement_length <= 0) {
        free(s);
        return NULL;
    }
    audio_jam_init(&s->state, seed);
    s->slots_per_section = audio_jam_slots_for_section(&s->base, section_seconds);
    if (s->slots_per_section < 1) s->slots_per_section = 1;
    return s;
}

unsigned char *audio_jam_session_render_next(AudioJamSession *s,
                                             int sample_rate, int *out_pcm_len)
{
    if (!s || !out_pcm_len) return NULL;
    SeqSong working;
    audio_jam_slice_song(&working, &s->base, s->state.arrangement_offset,
                         s->slots_per_section);
    audio_jam_vary_song(&working, &s->state);
    AudioRenderStats stats;
    unsigned char *pcm = audio_engine_render_song(&working, sample_rate,
                                                  out_pcm_len, &stats);
    s->state.arrangement_offset += s->slots_per_section;
    return pcm;
}

int audio_jam_session_iteration(const AudioJamSession *s)
{
    return s ? s->state.iteration : 0;
}

int audio_jam_session_arrangement_offset(const AudioJamSession *s)
{
    return s ? s->state.arrangement_offset : 0;
}

int audio_jam_session_slots_per_section(const AudioJamSession *s)
{
    return s ? s->slots_per_section : 0;
}

void audio_jam_session_close(AudioJamSession *s)
{
    free(s);
}
