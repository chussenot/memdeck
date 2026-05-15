#ifndef MEMDECK_AUDIO_FX_H
#define MEMDECK_AUDIO_FX_H

#include "audio_seq.h"

typedef struct {
    int enabled;
    int delay_steps;
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
    AudioSidechain sidechain;
} AudioFxBusState;

int audio_fx_delay_samples_from_steps(int sample_rate, int tempo_bpm, int steps_per_beat, int delay_steps);

int audio_fx_delay_init(AudioDelay *delay, int sample_rate, int delay_samples,
                        int feedback_percent, int mix_percent);
void audio_fx_delay_free(AudioDelay *delay);
void audio_fx_delay_reset(AudioDelay *delay);
int audio_fx_delay_process(AudioDelay *delay, int input);

int audio_fx_apply_drive(int input, int drive_amount);

void audio_fx_lowpass_init(AudioLowpass *lp, int amount);
int audio_fx_lowpass_process(AudioLowpass *lp, int input);

void audio_fx_sidechain_init(AudioSidechain *sc, int sample_rate, int amount, int release_ms);
int audio_fx_sidechain_process(AudioSidechain *sc, int input, int trigger, int *out_env_q15);

int audio_fx_bus_init(AudioFxBusState *state, const SeqFxBus *bus,
                      int sample_rate, int tempo_bpm, int steps_per_beat);
void audio_fx_bus_free(AudioFxBusState *state);
int audio_fx_bus_process(AudioFxBusState *state, int input, int trigger, int *out_sidechain_env_q15);

#endif
