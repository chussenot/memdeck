#ifndef MEMDECK_AUDIO_FX_H
#define MEMDECK_AUDIO_FX_H

#include "audio_seq.h"

typedef struct {
    int enabled;
    int delay_steps; /* runtime delay length in samples (kept for backward struct compatibility) */
    int feedback_percent;
    int mix_percent;
    int sample_rate;
    int write_index;
    int buffer_len;
    int *buffer;
} AudioDelay;

typedef struct {
    int enabled;
    int amount;
    int alpha_q15;
    int state;
} AudioLowpass;

/* Moog-style 4-pole ladder lowpass with resonance.
 * Q15 fixed-point transcription of the Stilson-Smith form (Stilson 1996,
 * Huovilainen 2004): four cascaded one-pole stages with a soft-clipped
 * resonance feedback from stage 4 back to the input. Self-oscillates near
 * resonance=100. */
typedef struct {
    int enabled;
    int amount;            /* wet mix percent (0-100); 0 short-circuits */
    int cutoff;            /* 1-100 percent of Nyquist */
    int resonance;         /* 0-100 */
    int alpha_q15;         /* one-pole coefficient (per stage) */
    int feedback_q15;      /* resonance feedback gain */
    int stage[4];          /* per-stage state, clamped to FX_BUFFER_CLAMP */
} AudioMoogLadder;

typedef struct {
    int enabled;
    int amount;
    int release_ms;
    int sample_rate;
    int env_q15;
    int release_step_q15;
} AudioSidechain;

typedef struct {
    int enabled;
    int mix_percent;
    int drive_amount;
    AudioDelay delay;
    AudioLowpass lowpass;
    AudioMoogLadder ladder;
    AudioSidechain sidechain;
} AudioFxBusState;

typedef struct {
    int edge_run;
    unsigned long long clipping_count;
} AudioClipStats;

int audio_fx_delay_samples_from_steps(int sample_rate, int tempo_bpm, int steps_per_beat, int delay_steps);

int audio_fx_delay_init(AudioDelay *delay, int sample_rate, int delay_samples,
                        int feedback_percent, int mix_percent);
void audio_fx_delay_free(AudioDelay *delay);
void audio_fx_delay_reset(AudioDelay *delay);
int audio_fx_delay_process(AudioDelay *delay, int input);

int audio_fx_apply_drive(int input, int drive_amount);

void audio_fx_lowpass_init(AudioLowpass *lp, int amount);
int audio_fx_lowpass_process(AudioLowpass *lp, int input);

void audio_fx_moog_ladder_init(AudioMoogLadder *m, int amount, int cutoff, int resonance);
int audio_fx_moog_ladder_process(AudioMoogLadder *m, int input);

void audio_fx_sidechain_init(AudioSidechain *sc, int sample_rate, int amount, int release_ms);
int audio_fx_sidechain_process(AudioSidechain *sc, int input, int trigger, int *out_env_q15);

int audio_fx_bus_init(AudioFxBusState *state, const SeqFxBus *bus,
                      int sample_rate, int tempo_bpm, int steps_per_beat);
void audio_fx_bus_free(AudioFxBusState *state);
int audio_fx_bus_process(AudioFxBusState *state, int input, int trigger, int *out_sidechain_env_q15);

void audio_fx_clip_stats_reset(AudioClipStats *stats);
void audio_fx_clip_stats_push(AudioClipStats *stats, unsigned char sample);

#endif
