#include "audio_mix.h"
#include "audio_dsp.h"

#include <stdlib.h>
#include <string.h>

typedef struct {
    int active;
    int note_end;
    int fx_bus;
    int fx_trigger;
} MixTrackStep;

typedef struct {
    DspOscillator osc;
    int ready;
    int instrument;
    int note;
    int amplitude;
} MixTrackState;

static int clampi(int value, int lo, int hi)
{
    if (value < lo) return lo;
    if (value > hi) return hi;
    return value;
}

static double midi_note_to_freq(int note)
{
    static const double midi_freq_table[128] = {
        8.175799, 8.661957, 9.177024, 9.722718, 10.300861, 10.913382, 11.562326, 12.249857,
        12.978272, 13.750000, 14.567618, 15.433853, 16.351598, 17.323914, 18.354048, 19.445436,
        20.601722, 21.826764, 23.124651, 24.499715, 25.956544, 27.500000, 29.135235, 30.867706,
        32.703196, 34.647829, 36.708096, 38.890873, 41.203445, 43.653529, 46.249303, 48.999429,
        51.913087, 55.000000, 58.270470, 61.735413, 65.406391, 69.295658, 73.416192, 77.781746,
        82.406889, 87.307058, 92.498606, 97.998859, 103.826174, 110.000000, 116.540940, 123.470825,
        130.812783, 138.591315, 146.832384, 155.563492, 164.813778, 174.614116, 184.997211, 195.997718,
        207.652349, 220.000000, 233.081881, 246.941651, 261.625565, 277.182631, 293.664768, 311.126984,
        329.627557, 349.228231, 369.994423, 391.995436, 415.304698, 440.000000, 466.163762, 493.883301,
        523.251131, 554.365262, 587.329536, 622.253967, 659.255114, 698.456463, 739.988845, 783.990872,
        830.609395, 880.000000, 932.327523, 987.766603, 1046.502261, 1108.730524, 1174.659072, 1244.507935,
        1318.510228, 1396.912926, 1479.977691, 1567.981744, 1661.218790, 1760.000000, 1864.655046, 1975.533205,
        2093.004522, 2217.461048, 2349.318143, 2489.015870, 2637.020455, 2793.825851, 2959.955382, 3135.963488,
        3322.437581, 3520.000000, 3729.310092, 3951.066410, 4186.009045, 4434.922096, 4698.636287, 4978.031740,
        5274.040911, 5587.651703, 5919.910763, 6271.926976, 6644.875161, 7040.000000, 7458.620184, 7902.132820,
        8372.018090, 8869.844191, 9397.272573, 9956.063479, 10548.081821, 11175.303406, 11839.821527, 12543.853951
    };

    if (note < 0 || note > 127) return 0.0;
    return midi_freq_table[note];
}

static int track_amplitude(const SeqSong *song, const SeqTrack *track,
                           const SeqStep *step, int pattern_step)
{
    int amplitude;
    const SeqInstrument *instrument = &song->instruments[track->instrument];

    amplitude = (instrument->amplitude * step->velocity) / 127;
    amplitude = (amplitude * (100 + track->automation[pattern_step])) / 100;
    if (step->accent)
        amplitude = (amplitude * (100 + instrument->accent_gain)) / 100;
    return clampi(amplitude, 0, 127);
}

static int track_gate(const SeqInstrument *instrument, const SeqStep *step)
{
    int gate = step->gate > 0 ? step->gate : instrument->envelope_gate;
    return clampi(gate, 1, 100);
}

unsigned char *audio_mix_render_timeline(const SeqSong *song, const SeqTimeline *timeline,
                                         int sample_rate, int *out_len)
{
    unsigned char *buf;
    MixTrackState track_state[SEQ_MAX_TRACKS];
    MixTrackStep step_state[SEQ_MAX_TRACKS];
    int fx_bus_levels[SEQ_MAX_FX_BUSES];
    int base = 0;

    if (!song || !timeline || !out_len || timeline->total_steps <= 0 || timeline->total_samples <= 0)
        return NULL;

    buf = malloc((size_t)timeline->total_samples);
    if (!buf) return NULL;

    memset(buf, 128, (size_t)timeline->total_samples);
    memset(track_state, 0, sizeof(track_state));

    for (int absolute_step = 0; absolute_step < timeline->total_steps; absolute_step++) {
        const SeqTimelineStep *timeline_step = &timeline->steps[absolute_step];
        const SeqPattern *pattern = &song->patterns[timeline_step->pattern_index];
        int samples_this_step = timeline_step->samples;

        memset(step_state, 0, sizeof(step_state));
        for (int track_index = 0; track_index < timeline->max_track_count; track_index++) {
            const SeqTrack *track;
            const SeqStep *step;
            const SeqInstrument *instrument;
            int amplitude;

            if (track_index >= pattern->track_count) {
                track_state[track_index].ready = 0;
                continue;
            }

            track = &pattern->tracks[track_index];
            if (track->instrument < 0 || track->instrument >= song->instrument_count) {
                track_state[track_index].ready = 0;
                continue;
            }

            step = &track->steps[timeline_step->pattern_step];
            if (step->note == SEQ_NOTE_REST || step->velocity == 0) {
                track_state[track_index].ready = 0;
                continue;
            }

            instrument = &song->instruments[track->instrument];
            amplitude = track_amplitude(song, track, step, timeline_step->pattern_step);
            step_state[track_index].active = 1;
            step_state[track_index].note_end = (samples_this_step * track_gate(instrument, step)) / 100;
            step_state[track_index].fx_bus = instrument->fx_send;
            step_state[track_index].fx_trigger = step->fx_trigger;

            if (!track_state[track_index].ready ||
                track_state[track_index].instrument != track->instrument ||
                track_state[track_index].note != step->note ||
                track_state[track_index].amplitude != amplitude) {
                dsp_osc_init(&track_state[track_index].osc,
                             (DspWaveform)instrument->oscillator, amplitude);
                dsp_osc_set_frequency(&track_state[track_index].osc,
                                      midi_note_to_freq(step->note), sample_rate);
                if (instrument->oscillator == DSP_WAVE_PULSE)
                    dsp_osc_set_pulse_width_percent(&track_state[track_index].osc,
                                                    clampi(instrument->duty_cycle, 1, 99));
                track_state[track_index].ready = 1;
                track_state[track_index].instrument = track->instrument;
                track_state[track_index].note = step->note;
                track_state[track_index].amplitude = amplitude;
            }
        }

        for (int i = 0; i < samples_this_step; i++) {
            int val = 128;

            memset(fx_bus_levels, 0, sizeof(fx_bus_levels));
            for (int track_index = 0; track_index < timeline->max_track_count; track_index++) {
                int sample;
                int fx_bus;
                if (!step_state[track_index].active || i >= step_state[track_index].note_end)
                    continue;
                sample = dsp_osc_next(&track_state[track_index].osc);
                val += sample;

                fx_bus = step_state[track_index].fx_bus;
                if (step_state[track_index].fx_trigger > 0 &&
                    fx_bus >= 0 && fx_bus < song->fx_bus_count) {
                    const SeqFxBus *bus = &song->fx_buses[fx_bus];
                    int drive = clampi(bus->drive, 0, 200);
                    fx_bus_levels[fx_bus] += (sample * step_state[track_index].fx_trigger * drive) / 10000;
                }
            }

            for (int bus_index = 0; bus_index < song->fx_bus_count; bus_index++) {
                const SeqFxBus *bus = &song->fx_buses[bus_index];
                val += (fx_bus_levels[bus_index] * clampi(bus->mix_percent, 0, 100)) / 100;
            }

            buf[base + i] = (unsigned char)dsp_clamp_u8(val);
        }
        base += samples_this_step;
    }

    *out_len = timeline->total_samples;
    return buf;
}

unsigned char *audio_mix_render_song(const SeqSong *song, int sample_rate, int *out_len)
{
    SeqTimeline timeline;

    if (seq_compile_timeline(song, sample_rate, &timeline) != 0)
        return NULL;
    return audio_mix_render_timeline(song, &timeline, sample_rate, out_len);
}
