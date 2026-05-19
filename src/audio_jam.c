#include "audio_jam.h"

#include <ctype.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "memdeck.h"
#include "audio_engine.h"

/* Lowercase substring check (case-insensitive), bounded so we never
 * read past the voice/instrument name buffer. */
static int name_contains(const char *name, const char *needle)
{
    if (!name || !needle) return 0;
    size_t nlen = strlen(needle);
    if (nlen == 0) return 0;
    for (size_t i = 0; name[i] != '\0' && i < 64; i++) {
        size_t j = 0;
        while (j < nlen) {
            char a = name[i + j];
            if (a == '\0') break;
            char la = (a >= 'A' && a <= 'Z') ? (char)(a + 32) : a;
            if (la != needle[j]) break;
            j++;
        }
        if (j == nlen) return 1;
    }
    return 0;
}

/* Voice/instrument naming is the strongest signal — every showcase demo
 * uses one of these conventions, and it survives key-signature pitch
 * shifts that confused a purely parametric classifier. */
static JamVoiceRole role_from_name(const char *name)
{
    if (!name || !name[0]) return JAM_VOICE_UNKNOWN;
    if (name_contains(name, "kick") || name_contains(name, "bd"))
        return JAM_VOICE_KICK;
    if (name_contains(name, "snare") || name_contains(name, "clap") ||
        name_contains(name, "hat")   || name_contains(name, "hh")   ||
        name_contains(name, "tom")   || name_contains(name, "perc"))
        return JAM_VOICE_SNARE_HAT;
    if (name_contains(name, "bass"))
        return JAM_VOICE_BASS;
    if (name_contains(name, "pad"))
        return JAM_VOICE_PAD;
    if (name_contains(name, "lead")  || name_contains(name, "arp")    ||
        name_contains(name, "stab")  || name_contains(name, "pluck")  ||
        name_contains(name, "vocal") || name_contains(name, "voc")    ||
        name_contains(name, "rhythm")|| name_contains(name, "guitar") ||
        name_contains(name, "synth") || name_contains(name, "hook"))
        return JAM_VOICE_LEAD;
    return JAM_VOICE_UNKNOWN;
}

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
    memset(jam, 0, sizeof(*jam));
    jam->state = seed ? seed : 0xCAFEBABE12345678ull;
    for (int t = 0; t < SEQ_MAX_TRACKS; t++) {
        jam->roles[t] = JAM_VOICE_UNKNOWN;
        jam->reference_note[t] = 60;
    }
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

/* Find the AbcInstrument referenced by a voice by name match. Returns
 * NULL if the voice didn't declare an instrument or the name doesn't
 * resolve — fall back to LEAD in that case. */
static const AbcInstrument *find_instrument(const AbcMusic *music, const AbcVoice *voice)
{
    if (!music || !voice || voice->instrument_ref[0] == '\0') return NULL;
    for (int i = 0; i < music->instrument_count && i < ABC_MAX_INSTRUMENTS; i++) {
        if (strcmp(music->instruments[i].name, voice->instrument_ref) == 0)
            return &music->instruments[i];
    }
    return NULL;
}

/* Walk the base SeqSong's tracks for this voice and pick the first
 * sounding step's MIDI note. Drum voices in this codebase typically
 * play the same note throughout (the "kick note"); melodic voices
 * vary, but we only use this when synthesising drum fills. */
static int first_note_for_track(const SeqSong *base, int track)
{
    if (!base) return 60;
    for (int p = 0; p < base->pattern_count && p < SEQ_MAX_PATTERNS; p++) {
        const SeqPattern *pat = &base->patterns[p];
        if (track >= pat->track_count) continue;
        const SeqTrack *trk = &pat->tracks[track];
        for (int s = 0; s < pat->length && s < SEQ_MAX_STEPS; s++) {
            if (trk->steps[s].velocity > 0)
                return trk->steps[s].note;
        }
    }
    return 60;
}

/* Mean MIDI note across all sounding steps of a voice in `base`. */
static int mean_note_for_track(const SeqSong *base, int track)
{
    int sum = 0;
    int count = 0;
    for (int p = 0; p < base->pattern_count && p < SEQ_MAX_PATTERNS; p++) {
        const SeqPattern *pat = &base->patterns[p];
        if (track >= pat->track_count) continue;
        const SeqTrack *trk = &pat->tracks[track];
        for (int s = 0; s < pat->length && s < SEQ_MAX_STEPS; s++) {
            if (trk->steps[s].velocity > 0) {
                sum += trk->steps[s].note;
                count++;
            }
        }
    }
    return count > 0 ? sum / count : 60;
}

void audio_jam_analyze_song(JamState *jam, const AbcMusic *music, const SeqSong *base)
{
    if (!jam || !music || !base) return;

    /* Classify each voice. */
    int track_count = base->pattern_count > 0
        ? base->patterns[0].track_count
        : music->voice_count;
    if (track_count > SEQ_MAX_TRACKS) track_count = SEQ_MAX_TRACKS;

    for (int t = 0; t < track_count; t++) {
        const AbcVoice *voice = (t < music->voice_count) ? &music->voices[t] : NULL;
        const AbcInstrument *inst = find_instrument(music, voice);
        int mean = mean_note_for_track(base, t);
        jam->reference_note[t] = first_note_for_track(base, t);

        /* Name-based first — strongest signal and survives accidentals. */
        JamVoiceRole role = JAM_VOICE_UNKNOWN;
        if (voice) role = role_from_name(voice->name);
        if (role == JAM_VOICE_UNKNOWN && inst) role = role_from_name(inst->name);

        if (role == JAM_VOICE_UNKNOWN) {
            /* Fallback: derive from instrument params + pitch. */
            if (inst && inst->waveform == 3 /* noise */) {
                int short_envelope = (inst->decay_ms <= 14 && inst->release_ms <= 20);
                /* Pitch is mostly cosmetic for noise — use envelope to tell
                 * tom-ish hits (longer) from hat-ish ones. */
                role = short_envelope ? JAM_VOICE_SNARE_HAT : JAM_VOICE_KICK;
            } else if (inst && inst->attack_ms >= 80 && inst->release_ms >= 200) {
                role = JAM_VOICE_PAD;
            } else if (mean > 0 && mean < 50) {
                role = JAM_VOICE_BASS;
            } else {
                role = JAM_VOICE_LEAD;
            }
        }
        jam->roles[t] = role;
    }
    for (int t = track_count; t < SEQ_MAX_TRACKS; t++) {
        jam->roles[t] = JAM_VOICE_UNKNOWN;
        jam->reference_note[t] = 60;
    }

    /* Per-pattern density. */
    for (int p = 0; p < SEQ_MAX_PATTERNS; p++) jam->pattern_density[p] = 0;
    for (int p = 0; p < base->pattern_count && p < SEQ_MAX_PATTERNS; p++) {
        const SeqPattern *pat = &base->patterns[p];
        int active = 0;
        int total = 0;
        for (int t = 0; t < pat->track_count && t < SEQ_MAX_TRACKS; t++) {
            const SeqTrack *trk = &pat->tracks[t];
            for (int s = 0; s < pat->length; s++) {
                total++;
                if (trk->steps[s].velocity > 0) active++;
            }
        }
        jam->pattern_density[p] = total > 0 ? (active * 100) / total : 0;
    }
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

/* Per-role velocity jitter magnitude. Drums and lead can take wider
 * humanisation; bass and pad should stay steady or they'll feel uneven. */
static int velocity_magnitude_for_role(JamVoiceRole role)
{
    switch (role) {
        case JAM_VOICE_KICK:      return 6;
        case JAM_VOICE_SNARE_HAT: return 10;
        case JAM_VOICE_BASS:      return 4;
        case JAM_VOICE_PAD:       return 2;
        case JAM_VOICE_LEAD:      return 8;
        default:                  return 8;
    }
}

static void perturb_velocities(SeqSong *song, JamState *jam)
{
    /* Jitter velocity on every sounding step. Magnitude scales by the
     * voice's role: drums can swing wider, pads stay steady. */
    for (int p = 0; p < song->pattern_count; p++) {
        SeqPattern *pat = &song->patterns[p];
        for (int t = 0; t < pat->track_count && t < SEQ_MAX_TRACKS; t++) {
            int magnitude = velocity_magnitude_for_role(jam->roles[t]);
            SeqTrack *track = &pat->tracks[t];
            for (int s = 0; s < pat->length && s < SEQ_MAX_STEPS; s++) {
                SeqStep *step = &track->steps[s];
                if (step->velocity == 0) continue;
                int delta = audio_jam_rand_range(jam, -magnitude, magnitude + 1);
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
    /* Swap two random adjacent slots, but only if their pattern
     * densities are close. Refusing to swap a peak chorus with a
     * silent breakdown keeps the macro-form recognisable. */
    int a = audio_jam_rand_range(jam, 0, song->arrangement_length - 1);
    int b = a + 1;
    int p_a = song->arrangement[a];
    int p_b = song->arrangement[b];
    if (p_a >= 0 && p_a < SEQ_MAX_PATTERNS && p_b >= 0 && p_b < SEQ_MAX_PATTERNS) {
        int delta = jam->pattern_density[p_a] - jam->pattern_density[p_b];
        if (delta < 0) delta = -delta;
        /* 35 = roughly "one section apart" given that demo densities
         * cluster around 25 (sparse) / 60 (medium) / 90 (peak). */
        if (delta > 35) return;
    }
    uint8_t tmp = song->arrangement[a];
    song->arrangement[a] = song->arrangement[b];
    song->arrangement[b] = tmp;
}

/* Pick a drum track for fills. Prefers KICK; falls back to SNARE_HAT;
 * returns -1 if there's no drum voice (a melodic-only piece). */
static int pick_drum_track(const JamState *jam)
{
    for (int t = 0; t < SEQ_MAX_TRACKS; t++)
        if (jam->roles[t] == JAM_VOICE_KICK) return t;
    for (int t = 0; t < SEQ_MAX_TRACKS; t++)
        if (jam->roles[t] == JAM_VOICE_SNARE_HAT) return t;
    return -1;
}

static void drum_fill_last_bar(SeqSong *song, JamState *jam)
{
    if (song->pattern_count <= 0) return;
    int drum_track = pick_drum_track(jam);
    if (drum_track < 0) return; /* nothing to fill */

    int p_idx = audio_jam_rand_range(jam, 0, song->pattern_count);
    SeqPattern *pat = &song->patterns[p_idx];
    if (drum_track >= pat->track_count) return;
    SeqTrack *track = &pat->tracks[drum_track];

    int spb = song->steps_per_beat > 0 ? song->steps_per_beat : 4;
    int bar_steps = spb * 4; /* assume 4/4 — true for every showcase song */
    if (bar_steps > pat->length) bar_steps = pat->length;
    int start = pat->length - bar_steps;
    if (start < 0) start = 0;

    /* Use the voice's own reference note so the fill blends with the
     * existing kicks/snares instead of bursting an alien pitch. */
    int note = jam->reference_note[drum_track];
    if (note <= 0 || note > 127) note = 60;

    for (int s = start; s < pat->length; s++) {
        SeqStep *step = &track->steps[s];
        step->note = (int16_t)note;
        step->velocity = 96;
        step->gate = 70;
        step->accent = (uint8_t)((s == start) ? 1 : 0);
        step->fx_trigger = step->accent;
    }
}

/* Weighted track pick for mute: hats and lead are the cheap targets,
 * bass and kick almost never go silent. */
static int mute_weight_for_role(JamVoiceRole role)
{
    switch (role) {
        case JAM_VOICE_SNARE_HAT: return 5;
        case JAM_VOICE_LEAD:      return 4;
        case JAM_VOICE_PAD:       return 3;
        case JAM_VOICE_KICK:      return 1; /* drops are rare */
        case JAM_VOICE_BASS:      return 1; /* bass-less feels broken */
        default:                  return 2;
    }
}

static void mute_voice_in_pattern(SeqSong *song, JamState *jam)
{
    if (song->pattern_count <= 0) return;
    int p_idx = audio_jam_rand_range(jam, 0, song->pattern_count);
    SeqPattern *pat = &song->patterns[p_idx];
    if (pat->track_count <= 0) return;

    int total = 0;
    int weights[SEQ_MAX_TRACKS] = {0};
    for (int t = 0; t < pat->track_count && t < SEQ_MAX_TRACKS; t++) {
        weights[t] = mute_weight_for_role(jam->roles[t]);
        total += weights[t];
    }
    if (total <= 0) return;
    int r = audio_jam_rand_range(jam, 0, total);
    int acc = 0;
    int chosen = 0;
    for (int t = 0; t < pat->track_count && t < SEQ_MAX_TRACKS; t++) {
        acc += weights[t];
        if (r < acc) { chosen = t; break; }
    }

    SeqTrack *track = &pat->tracks[chosen];
    for (int s = 0; s < pat->length; s++) {
        track->steps[s].velocity = 0;
        track->steps[s].gate = 0;
    }
}

/* Lift one pattern's notes for a lead voice up or down an octave. Cheap
 * way to inject melodic variation without inventing new pitches. */
static void octave_shift_lead(SeqSong *song, JamState *jam)
{
    int lead_track = -1;
    for (int t = 0; t < SEQ_MAX_TRACKS; t++) {
        if (jam->roles[t] == JAM_VOICE_LEAD) { lead_track = t; break; }
    }
    if (lead_track < 0 || song->pattern_count <= 0) return;

    int p_idx = audio_jam_rand_range(jam, 0, song->pattern_count);
    SeqPattern *pat = &song->patterns[p_idx];
    if (lead_track >= pat->track_count) return;
    int direction = (audio_jam_rand_range(jam, 0, 2) == 0) ? -12 : 12;

    SeqTrack *track = &pat->tracks[lead_track];
    for (int s = 0; s < pat->length; s++) {
        if (track->steps[s].velocity == 0) continue;
        int n = (int)track->steps[s].note + direction;
        /* Skip steps that would shoot out of MIDI range — leave them as-is. */
        if (n < 0 || n > 127) continue;
        track->steps[s].note = (int16_t)n;
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

    if (audio_jam_rand_range(jam, 0, 100) < 25)
        octave_shift_lead(song, jam);
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
    audio_jam_analyze_song(&s->state, &s->music, &s->base);
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
