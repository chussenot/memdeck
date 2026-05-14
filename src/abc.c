#include "memdeck.h"
#include "audio_dsp.h"

/*
 * Minimal ABC notation parser for the MemDeck chiptune engine.
 *
 * Supports:
 *   - Header fields: X, T, M, L, Q, K, V (with amp= and staccato directives)
 *   - Notes: C-B (octave 4), c-b (octave 5), with , (down) and ' (up) modifiers
 *   - Accidentals: ^ (sharp), _ (flat), = (natural)
 *   - Rests: z
 *   - Note lengths: multipliers (e.g. A2 = twice default length)
 *   - Repeat markers: |: and :|
 *   - Multi-voice via V: lines
 */

/* ─── Note frequency table ───────────────────────────────────── */

/* semitone offsets from C: C=0, D=2, E=4, F=5, G=7, A=9, B=11 */
static const int note_semitones[7] = { 0, 2, 4, 5, 7, 9, 11 };

/*
 * Precomputed MIDI note frequencies (Hz).
 * Index = MIDI note number (0–127). A4 = index 69 = 440.000000 Hz.
 * Generated via: 440.0 * pow(2.0, (i - 69) / 12.0)
 * Eliminates per-note pow() calls during parsing.
 */
static const double midi_freq_table[128] = {
    8.175799, 8.661957, 9.177024, 9.722718,
    10.300861, 10.913382, 11.562326, 12.249857,
    12.978272, 13.750000, 14.567618, 15.433853,
    16.351598, 17.323914, 18.354048, 19.445436,
    20.601722, 21.826764, 23.124651, 24.499715,
    25.956544, 27.500000, 29.135235, 30.867706,
    32.703196, 34.647829, 36.708096, 38.890873,
    41.203445, 43.653529, 46.249303, 48.999429,
    51.913087, 55.000000, 58.270470, 61.735413,
    65.406391, 69.295658, 73.416192, 77.781746,
    82.406889, 87.307058, 92.498606, 97.998859,
    103.826174, 110.000000, 116.540940, 123.470825,
    130.812783, 138.591315, 146.832384, 155.563492,
    164.813778, 174.614116, 184.997211, 195.997718,
    207.652349, 220.000000, 233.081881, 246.941651,
    261.625565, 277.182631, 293.664768, 311.126984,
    329.627557, 349.228231, 369.994423, 391.995436,
    415.304698, 440.000000, 466.163762, 493.883301,
    523.251131, 554.365262, 587.329536, 622.253967,
    659.255114, 698.456463, 739.988845, 783.990872,
    830.609395, 880.000000, 932.327523, 987.766603,
    1046.502261, 1108.730524, 1174.659072, 1244.507935,
    1318.510228, 1396.912926, 1479.977691, 1567.981744,
    1661.218790, 1760.000000, 1864.655046, 1975.533205,
    2093.004522, 2217.461048, 2349.318143, 2489.015870,
    2637.020455, 2793.825851, 2959.955382, 3135.963488,
    3322.437581, 3520.000000, 3729.310092, 3951.066410,
    4186.009045, 4434.922096, 4698.636287, 4978.031740,
    5274.040911, 5587.651703, 5919.910763, 6271.926976,
    6644.875161, 7040.000000, 7458.620184, 7902.132820,
    8372.018090, 8869.844191, 9397.272573, 9956.063479,
    10548.081821, 11175.303406, 11839.821527, 12543.853951
};

static double semitone_to_freq(int midi_note)
{
    if (midi_note < 0 || midi_note > 127) return 0.0;
    return midi_freq_table[midi_note];
}

static int note_char_to_index(char c)
{
    /* C=0, D=1, E=2, F=3, G=4, A=5, B=6 */
    switch (c) {
        case 'C': case 'c': return 0;
        case 'D': case 'd': return 1;
        case 'E': case 'e': return 2;
        case 'F': case 'f': return 3;
        case 'G': case 'g': return 4;
        case 'A': case 'a': return 5;
        case 'B': case 'b': return 6;
    }
    return -1;
}

static DspWaveform sanitize_waveform(int waveform)
{
    if (waveform < DSP_WAVE_SQUARE || waveform > DSP_WAVE_NOISE)
        return DSP_WAVE_SQUARE;
    return (DspWaveform)waveform;
}

static int parse_waveform_name(const char *s)
{
    if (!s) return DSP_WAVE_SQUARE;
    if (strncmp(s, "pulse", 5) == 0) return DSP_WAVE_PULSE;
    if (strncmp(s, "triangle", 8) == 0) return DSP_WAVE_TRIANGLE;
    if (strncmp(s, "noise", 5) == 0) return DSP_WAVE_NOISE;
    return DSP_WAVE_SQUARE;
}

static void parse_voice_directives(AbcVoice *v, const char *val)
{
    const char *amp = strstr(val, "amp=");
    const char *wave = strstr(val, "wave=");
    const char *duty = strstr(val, "duty=");

    if (amp) v->amplitude = atoi(amp + 4);
    if (strstr(val, "staccato")) v->staccato = 1;
    if (wave) v->waveform = parse_waveform_name(wave + 5);
    if (duty) {
        int pct = atoi(duty + 5);
        if (pct < 1) pct = 1;
        if (pct > 99) pct = 99;
        v->duty_cycle = pct;
    }
}

/* ─── Parser state ───────────────────────────────────────────── */

typedef struct {
    int beats_per_bar;    /* M: numerator */
    int beat_unit;        /* M: denominator */
    int default_len_num;  /* L: numerator (e.g. 1) */
    int default_len_den;  /* L: denominator (e.g. 16) */
    int tempo_bpm;        /* Q: beats per minute */
    int key_sharps;       /* key signature accidentals (positive=sharps, negative=flats) */
    int key_accidentals[7]; /* per-note accidentals from key: -1=flat, 0=natural, 1=sharp */
} AbcHeader;

static void parse_key_sig(const char *key_str, AbcHeader *h)
{
    memset(h->key_accidentals, 0, sizeof(h->key_accidentals));

    /* Simple key parsing: major keys and Dm */
    char base = key_str[0];
    int is_minor = (strstr(key_str, "m") != NULL || strstr(key_str, "min") != NULL);

    /* Determine number of sharps/flats from key */
    /* Order of sharps: F C G D A E B */
    /* Order of flats:  B E A D G C F */
    static const int sharp_order[7] = { 3, 0, 4, 1, 5, 2, 6 }; /* F,C,G,D,A,E,B note indices */
    static const int flat_order[7]  = { 6, 2, 5, 1, 4, 0, 3 }; /* B,E,A,D,G,C,F */

    /* Map base note to number of sharps in major key (C=0, G=1, D=2...) */
    int nsharps = 0;
    switch (base) {
        case 'C': nsharps = 0; break;
        case 'G': nsharps = 1; break;
        case 'D': nsharps = 2; break;
        case 'A': nsharps = 3; break;
        case 'E': nsharps = 4; break;
        case 'B': nsharps = 5; break;
        case 'F':
            /* Check for F# */
            if (key_str[1] == '#') nsharps = 6;
            else nsharps = -1; /* F major = 1 flat */
            break;
        default: nsharps = 0;
    }

    /* Minor keys: relative minor is 3 semitones below major,
       which means 3 fewer sharps */
    if (is_minor) nsharps -= 3;

    /* Check for explicit flats/sharps after note name */
    if (key_str[1] == 'b' && key_str[1] != '\0') nsharps -= 1; /* e.g. Bb major */

    h->key_sharps = nsharps;

    if (nsharps > 0) {
        for (int i = 0; i < nsharps && i < 7; i++)
            h->key_accidentals[sharp_order[i]] = 1;
    } else if (nsharps < 0) {
        int nflats = -nsharps;
        for (int i = 0; i < nflats && i < 7; i++)
            h->key_accidentals[flat_order[i]] = -1;
    }
}

/* ─── Parse a single ABC note from text ──────────────────────── */

/*
 * Parse one note starting at *p. Returns the number of characters consumed.
 * Sets *freq to the note frequency and *steps to the number of default-length
 * units this note occupies.
 */
static int parse_note(const char *p, const AbcHeader *h,
                      double *freq, int *steps)
{
    const char *start = p;
    *freq = 0.0;
    *steps = 1;

    /* skip barlines, spaces, decorations */
    while (*p == '|' || *p == ':' || *p == ' ' || *p == '[' || *p == ']')
        p++;

    if (*p == '\0' || *p == '\n' || *p == '%') {
        *steps = 0;
        return (int)(p - start);
    }

    /* rest */
    if (*p == 'z' || *p == 'x') {
        p++;
        /* parse optional length multiplier */
        int mult = 0;
        while (*p >= '0' && *p <= '9') {
            mult = mult * 10 + (*p - '0');
            p++;
        }
        if (mult > 0) *steps = mult;
        *freq = 0.0;
        return (int)(p - start);
    }

    /* accidental */
    int accidental = -99; /* -99 = use key signature */
    if (*p == '^') { accidental = 1; p++; if (*p == '^') { accidental = 2; p++; } }
    else if (*p == '_') { accidental = -1; p++; if (*p == '_') { accidental = -2; p++; } }
    else if (*p == '=') { accidental = 0; p++; }

    /* note letter */
    char note_char = *p;
    int note_idx = note_char_to_index(note_char);
    if (note_idx < 0) {
        *steps = 0;
        return (int)(p - start);
    }

    /* ABC: uppercase C-B = octave 4 (C4-B4), lowercase c-b = octave 5 (C5-B5) */
    int octave = (note_char >= 'a' && note_char <= 'z') ? 5 : 4;
    p++;

    /* octave modifiers */
    while (*p == '\'') { octave++; p++; }
    while (*p == ',')  { octave--; p++; }

    /* compute MIDI note */
    int semitone = note_semitones[note_idx];
    if (accidental != -99) {
        semitone += accidental;
    } else {
        semitone += h->key_accidentals[note_idx];
    }
    int midi = (octave + 1) * 12 + semitone; /* C4 = (4+1)*12 + 0 = 60 */

    *freq = semitone_to_freq(midi);

    /* parse optional length multiplier */
    int mult = 0;
    while (*p >= '0' && *p <= '9') {
        mult = mult * 10 + (*p - '0');
        p++;
    }
    if (mult > 0) *steps = mult;

    /* handle fractional lengths (e.g. A/ = half, A/2 = half) */
    if (*p == '/') {
        p++;
        int div = 2;
        if (*p >= '0' && *p <= '9') {
            div = 0;
            while (*p >= '0' && *p <= '9') {
                div = div * 10 + (*p - '0');
                p++;
            }
        }
        /* fractional notes: round to at least 1 step */
        if (mult == 0) mult = 1;
        *steps = (mult > div) ? mult / div : 1;
    }

    return (int)(p - start);
}

/* ─── Public API ─────────────────────────────────────────────── */

int abc_load(const char *path, AbcMusic *music)
{
    FILE *f = fopen(path, "r");
    if (!f) return -1;

    memset(music, 0, sizeof(*music));
    music->voice_count = 0;
    music->bpm = 120;

    AbcHeader header;
    memset(&header, 0, sizeof(header));
    header.beats_per_bar = 4;
    header.beat_unit = 4;
    header.default_len_num = 1;
    header.default_len_den = 16;
    header.tempo_bpm = 120;

    int current_voice = -1;
    int in_repeat = 0;
    char repeat_buf[4096] = {0};
    int repeat_len = 0;

    char line[1024];
    while (fgets(line, sizeof(line), f)) {
        /* strip trailing newline */
        int len = (int)strlen(line);
        while (len > 0 && (line[len-1] == '\n' || line[len-1] == '\r'))
            line[--len] = '\0';

        /* skip comments */
        if (line[0] == '%' && line[1] != '%') continue;

        /* header fields */
        if (len >= 2 && line[1] == ':' && line[0] != '|') {
            char field = line[0];
            const char *val = line + 2;
            while (*val == ' ') val++;

            switch (field) {
                case 'X': break; /* reference number, ignored */
                case 'T': /* title */
                    snprintf(music->title, sizeof(music->title), "%.127s", val);
                    break;
                case 'M': /* meter */
                    sscanf(val, "%d/%d", &header.beats_per_bar, &header.beat_unit);
                    break;
                case 'L': /* default note length */
                    sscanf(val, "%d/%d", &header.default_len_num, &header.default_len_den);
                    break;
                case 'Q': { /* tempo */
                    int bpm = 120;
                    /* Q:1/4=120 or Q:120 */
                    const char *eq = strchr(val, '=');
                    if (eq) bpm = atoi(eq + 1);
                    else bpm = atoi(val);
                    if (bpm > 0) {
                        header.tempo_bpm = bpm;
                        music->bpm = bpm;
                    }
                    break;
                }
                case 'K': /* key */
                    parse_key_sig(val, &header);
                    break;
                case 'V': { /* voice */
                    /* Find or create voice by name */
                    char vname[32] = {0};
                    sscanf(val, "%31s", vname);

                    /* look for existing voice */
                    int found = -1;
                    for (int i = 0; i < music->voice_count; i++) {
                        if (strcmp(music->voices[i].name, vname) == 0) {
                            found = i;
                            break;
                        }
                    }

                    if (found >= 0) {
                        current_voice = found;
                    } else if (music->voice_count < ABC_MAX_VOICES) {
                        current_voice = music->voice_count++;
                        AbcVoice *v = &music->voices[current_voice];
                        memset(v, 0, sizeof(*v));
                        snprintf(v->name, sizeof(v->name), "%s", vname);
                        v->amplitude = 40; /* default */
                        v->staccato = 0;
                        v->waveform = DSP_WAVE_SQUARE;
                        v->duty_cycle = 25;
                    }

                    /* parse voice directives: amp=N, staccato */
                    AbcVoice *v = &music->voices[current_voice];
                    parse_voice_directives(v, val);
                    break;
                }
                default: break;
            }
            continue;
        }

        /* %%voice directive (pseudo-comment) */
        if (strncmp(line, "%%voice", 7) == 0) {
            const char *val = line + 7;
            while (*val == ' ') val++;

            char vname[32] = {0};
            sscanf(val, "%31s", vname);

            if (music->voice_count < ABC_MAX_VOICES) {
                current_voice = music->voice_count++;
                AbcVoice *v = &music->voices[current_voice];
                memset(v, 0, sizeof(*v));
                snprintf(v->name, sizeof(v->name), "%s", vname);
                v->amplitude = 40;
                v->staccato = 0;
                v->waveform = DSP_WAVE_SQUARE;
                v->duty_cycle = 25;

                parse_voice_directives(v, val);
            }
            continue;
        }

        /* skip other %% directives */
        if (line[0] == '%') continue;

        /* If no voice defined yet, create a default one */
        if (current_voice < 0) {
            if (music->voice_count < ABC_MAX_VOICES) {
                current_voice = music->voice_count++;
                AbcVoice *v = &music->voices[current_voice];
                memset(v, 0, sizeof(*v));
                snprintf(v->name, sizeof(v->name), "default");
                v->amplitude = 40;
                v->waveform = DSP_WAVE_SQUARE;
                v->duty_cycle = 25;
            } else {
                current_voice = 0;
            }
        }

        AbcVoice *v = &music->voices[current_voice];

        /* parse note data from this line */
        const char *p = line;
        while (*p) {
            /* skip whitespace and barlines */
            while (*p == ' ' || *p == '\t') p++;
            if (*p == '\0' || *p == '%') break;

            /* handle repeat start */
            if (*p == '|' && *(p+1) == ':') {
                in_repeat = 1;
                repeat_len = 0;
                repeat_buf[0] = '\0';
                p += 2;
                continue;
            }

            /* handle repeat end */
            if (*p == ':' && *(p+1) == '|') {
                if (in_repeat && repeat_len > 0) {
                    /* replay the repeated section */
                    const char *rp = repeat_buf;
                    while (*rp) {
                        double freq;
                        int steps;
                        int consumed = parse_note(rp, &header, &freq, &steps);
                        if (consumed == 0) { rp++; continue; }
                        rp += consumed;
                        for (int s = 0; s < steps && v->note_count < ABC_MAX_NOTES; s++) {
                            v->freqs[v->note_count] = freq;
                            v->note_count++;
                        }
                    }
                }
                in_repeat = 0;
                p += 2;
                continue;
            }

            /* handle simple barline */
            if (*p == '|') { p++; continue; }

            double freq;
            int steps;
            int consumed = parse_note(p, &header, &freq, &steps);
            if (consumed == 0) { p++; continue; }

            /* if in repeat section, also buffer the raw text */
            if (in_repeat) {
                int avail = (int)sizeof(repeat_buf) - repeat_len - 1;
                if (consumed < avail) {
                    memcpy(repeat_buf + repeat_len, p, consumed);
                    repeat_len += consumed;
                    repeat_buf[repeat_len++] = ' ';
                    repeat_buf[repeat_len] = '\0';
                }
            }

            p += consumed;

            for (int s = 0; s < steps && v->note_count < ABC_MAX_NOTES; s++) {
                v->freqs[v->note_count] = freq;
                v->note_count++;
            }
        }
    }

    fclose(f);

    /* compute step duration from tempo and default note length */
    /* default length in beats = default_len_num / default_len_den * beat_unit */
    /* step_ms = (60000 / bpm) * (default_len / quarter_note) */
    double quarter_ms = 60000.0 / music->bpm;
    double note_fraction = (double)header.default_len_num / header.default_len_den;
    /* note_fraction of a whole note; a quarter note = 1/4 */
    music->step_ms = (int)(quarter_ms * note_fraction * 4.0);
    if (music->step_ms < 10) music->step_ms = 125; /* fallback */

    return 0;
}

int abc_load_voices(const char *paths[], int path_count, AbcMusic *music)
{
    memset(music, 0, sizeof(*music));
    music->bpm = 120;
    music->step_ms = 125;

    for (int i = 0; i < path_count && i < ABC_MAX_VOICES; i++) {
        AbcMusic single;
        if (abc_load(paths[i], &single) != 0) continue;

        /* copy first voice from each file */
        if (single.voice_count > 0 && music->voice_count < ABC_MAX_VOICES) {
            music->voices[music->voice_count] = single.voices[0];
            music->voice_count++;
        }

        /* use tempo from first file */
        if (i == 0) {
            music->bpm = single.bpm;
            music->step_ms = single.step_ms;
            snprintf(music->title, sizeof(music->title), "%s", single.title);
        }
    }

    return (music->voice_count > 0) ? 0 : -1;
}

unsigned char *abc_generate_pcm(const AbcMusic *music, int *out_len)
{
    if (music->voice_count == 0) return NULL;

    /* find the longest voice (in steps) */
    int max_steps = 0;
    for (int v = 0; v < music->voice_count; v++) {
        if (music->voices[v].note_count > max_steps)
            max_steps = music->voices[v].note_count;
    }
    if (max_steps == 0) return NULL;

    int total_samples = dsp_total_samples_for_steps(SAMPLE_RATE_ABC, music->step_ms, max_steps);
    unsigned char *buf = malloc(total_samples);
    if (!buf) return NULL;

    memset(buf, 128, total_samples);
    DspSampleStepper stepper;
    dsp_stepper_init(&stepper, SAMPLE_RATE_ABC, music->step_ms);
    int base = 0;

    for (int step = 0; step < max_steps; step++) {
        int step_samples = dsp_stepper_next(&stepper);
        DspOscillator oscs[ABC_MAX_VOICES];
        int voice_on[ABC_MAX_VOICES] = {0};
        int note_end[ABC_MAX_VOICES] = {0};

        for (int v = 0; v < music->voice_count; v++) {
            const AbcVoice *voice = &music->voices[v];
            if (step >= voice->note_count) continue;
            if (voice->freqs[step] <= 0.0) continue;

            DspWaveform wf = sanitize_waveform(voice->waveform);
            dsp_osc_init(&oscs[v], wf, voice->amplitude);
            dsp_osc_set_frequency(&oscs[v], voice->freqs[step], SAMPLE_RATE_ABC);
            if (wf == DSP_WAVE_PULSE)
                dsp_osc_set_pulse_width_percent(&oscs[v], voice->duty_cycle);
            note_end[v] = voice->staccato
                ? (step_samples * 3) / 4
                : (step_samples * 9) / 10;
            voice_on[v] = 1;
        }

        for (int i = 0; i < step_samples; i++) {
            int val = 128;

            for (int v = 0; v < music->voice_count; v++) {
                if (!voice_on[v] || i >= note_end[v]) continue;
                val += dsp_osc_next(&oscs[v]);
            }

            buf[base + i] = (unsigned char)dsp_clamp_u8(val);
        }
        base += step_samples;
    }

    *out_len = total_samples;
    return buf;
}
