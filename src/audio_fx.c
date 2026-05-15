#include "audio_fx.h"

#include <stdlib.h>
#include <string.h>

#define FX_Q15_ONE 32767
#define FX_BUFFER_CLAMP 32767
#define FX_DRIVE_SOFT_CLIP 192
#define FX_DRIVE_HARD_CLIP 2047

static int clampi(int value, int lo, int hi)
{
    if (value < lo) return lo;
    if (value > hi) return hi;
    return value;
}

static int clamp_sample(int value, int limit)
{
    if (value < -limit) return -limit;
    if (value > limit) return limit;
    return value;
}

int audio_fx_delay_samples_from_steps(int sample_rate, int tempo_bpm, int steps_per_beat, int delay_steps)
{
    int denom;
    int step_samples;
    if (sample_rate <= 0 || tempo_bpm <= 0 || steps_per_beat <= 0 || delay_steps <= 0)
        return 0;
    denom = tempo_bpm * steps_per_beat;
    step_samples = (sample_rate * 60 + denom / 2) / denom;
    if (step_samples <= 0) return 0;
    return step_samples * delay_steps;
}

int audio_fx_delay_init(AudioDelay *delay, int sample_rate, int delay_samples,
                        int feedback_percent, int mix_percent)
{
    int len;
    if (!delay) return -1;
    memset(delay, 0, sizeof(*delay));
    if (delay_samples <= 0 || mix_percent <= 0)
        return 0;
    len = clampi(delay_samples, 1, sample_rate * 8);
    delay->buffer = calloc((size_t)len, sizeof(int));
    if (!delay->buffer) return -1;
    delay->enabled = 1;
    delay->delay_steps = delay_samples;
    delay->feedback_percent = clampi(feedback_percent, 0, 95);
    delay->mix_percent = clampi(mix_percent, 0, 100);
    delay->sample_rate = sample_rate;
    delay->buffer_len = len;
    delay->write_index = 0;
    return 0;
}

void audio_fx_delay_free(AudioDelay *delay)
{
    if (!delay) return;
    free(delay->buffer);
    memset(delay, 0, sizeof(*delay));
}

void audio_fx_delay_reset(AudioDelay *delay)
{
    if (!delay || !delay->buffer) return;
    memset(delay->buffer, 0, sizeof(int) * (size_t)delay->buffer_len);
    delay->write_index = 0;
}

int audio_fx_delay_process(AudioDelay *delay, int input)
{
    int delayed;
    int dry;
    int wet;
    int fb;
    if (!delay || !delay->enabled || !delay->buffer || delay->buffer_len <= 0)
        return input;

    delayed = delay->buffer[delay->write_index];
    dry = (input * (100 - delay->mix_percent)) / 100;
    wet = (delayed * delay->mix_percent) / 100;
    fb = input + (delayed * delay->feedback_percent) / 100;
    delay->buffer[delay->write_index] = clamp_sample(fb, FX_BUFFER_CLAMP);
    delay->write_index++;
    if (delay->write_index >= delay->buffer_len)
        delay->write_index = 0;
    return dry + wet;
}

int audio_fx_apply_drive(int input, int drive_amount)
{
    int gain;
    int x;
    int sign = 1;
    int a;
    int clipped;

    drive_amount = clampi(drive_amount, 0, 100);
    if (drive_amount <= 0)
        return clamp_sample(input, FX_DRIVE_HARD_CLIP);

    gain = 100 + drive_amount * 3;
    x = (input * gain) / 100;
    if (x < 0) {
        sign = -1;
        x = -x;
    }
    a = x;
    if (a <= FX_DRIVE_SOFT_CLIP) return sign * a;

    clipped = FX_DRIVE_SOFT_CLIP + ((a - FX_DRIVE_SOFT_CLIP) * 256) / ((a - FX_DRIVE_SOFT_CLIP) + 256);
    return clamp_sample(sign * clipped, FX_DRIVE_HARD_CLIP);
}

void audio_fx_lowpass_init(AudioLowpass *lp, int amount)
{
    if (!lp) return;
    memset(lp, 0, sizeof(*lp));
    amount = clampi(amount, 0, 100);
    if (amount <= 0) return;
    lp->enabled = 1;
    lp->amount = amount;
    lp->alpha_q15 = 24576 - (amount * 220);
    lp->alpha_q15 = clampi(lp->alpha_q15, 2048, 24576);
}

int audio_fx_lowpass_process(AudioLowpass *lp, int input)
{
    if (!lp || !lp->enabled) return input;
    lp->state += ((input - lp->state) * lp->alpha_q15) / FX_Q15_ONE;
    return lp->state;
}

void audio_fx_sidechain_init(AudioSidechain *sc, int sample_rate, int amount, int release_ms)
{
    int release_samples;
    if (!sc) return;
    memset(sc, 0, sizeof(*sc));
    amount = clampi(amount, 0, 100);
    if (amount <= 0) {
        sc->env_q15 = FX_Q15_ONE;
        return;
    }
    sc->enabled = 1;
    sc->amount = amount;
    sc->release_ms = clampi(release_ms, 10, 2000);
    sc->sample_rate = sample_rate;
    sc->env_q15 = FX_Q15_ONE;
    release_samples = (sample_rate * sc->release_ms) / 1000;
    if (release_samples <= 0) release_samples = 1;
    sc->release_step_q15 = FX_Q15_ONE / release_samples;
    if (sc->release_step_q15 < 1) sc->release_step_q15 = 1;
}

int audio_fx_sidechain_process(AudioSidechain *sc, int input, int trigger, int *out_env_q15)
{
    int duck_q15;
    if (!sc || !sc->enabled) {
        if (out_env_q15) *out_env_q15 = FX_Q15_ONE;
        return input;
    }

    if (trigger > 0) {
        int target;
        duck_q15 = (sc->amount * FX_Q15_ONE) / 100;
        target = FX_Q15_ONE - duck_q15;
        if (sc->env_q15 > target)
            sc->env_q15 = target;
    }

    input = (input * sc->env_q15) / FX_Q15_ONE;
    sc->env_q15 += sc->release_step_q15;
    if (sc->env_q15 > FX_Q15_ONE)
        sc->env_q15 = FX_Q15_ONE;
    if (out_env_q15) *out_env_q15 = sc->env_q15;
    return input;
}

int audio_fx_bus_init(AudioFxBusState *state, const SeqFxBus *bus,
                      int sample_rate, int tempo_bpm, int steps_per_beat)
{
    int delay_samples;
    if (!state || !bus) return -1;
    memset(state, 0, sizeof(*state));
    state->enabled = bus->enabled ? 1 : 0;
    state->mix_percent = clampi(bus->mix_percent, 0, 100);
    state->drive_amount = clampi(bus->drive_amount, 0, 100);
    audio_fx_lowpass_init(&state->lowpass, bus->lowpass_amount);
    audio_fx_sidechain_init(&state->sidechain, sample_rate, bus->sidechain_amount, bus->sidechain_release_ms);

    delay_samples = audio_fx_delay_samples_from_steps(sample_rate, tempo_bpm, steps_per_beat, bus->delay_steps);
    if (audio_fx_delay_init(&state->delay, sample_rate, delay_samples, bus->delay_feedback, bus->delay_mix) != 0) {
        memset(state, 0, sizeof(*state));
        return -1;
    }
    return 0;
}

void audio_fx_bus_free(AudioFxBusState *state)
{
    if (!state) return;
    audio_fx_delay_free(&state->delay);
    memset(state, 0, sizeof(*state));
}

int audio_fx_bus_process(AudioFxBusState *state, int input, int trigger, int *out_sidechain_env_q15)
{
    int sample = input;
    int env_q15 = FX_Q15_ONE;

    if (!state || !state->enabled) {
        if (out_sidechain_env_q15) *out_sidechain_env_q15 = FX_Q15_ONE;
        return 0;
    }

    sample = audio_fx_apply_drive(sample, state->drive_amount);
    sample = audio_fx_lowpass_process(&state->lowpass, sample);
    sample = audio_fx_delay_process(&state->delay, sample);
    sample = audio_fx_sidechain_process(&state->sidechain, sample, trigger, &env_q15);
    sample = (sample * state->mix_percent) / 100;

    if (out_sidechain_env_q15) *out_sidechain_env_q15 = env_q15;
    return sample;
}
