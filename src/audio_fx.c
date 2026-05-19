#include "audio_fx.h"

#include <stdlib.h>
#include <string.h>

#define FX_Q15_ONE 32767
#define FX_BUFFER_CLAMP 32767
#define FX_DRIVE_SOFT_CLIP 192
#define FX_DRIVE_HARD_CLIP 2047

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
    len = dsp_clampi(delay_samples, 1, sample_rate * 8);
    delay->buffer = calloc((size_t)len, sizeof(int));
    if (!delay->buffer) return -1;
    delay->enabled = 1;
    delay->delay_steps = delay_samples;
    delay->feedback_percent = dsp_clampi(feedback_percent, 0, 95);
    delay->mix_percent = dsp_clampi(mix_percent, 0, 100);
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

    drive_amount = dsp_clampi(drive_amount, 0, 100);
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
    amount = dsp_clampi(amount, 0, 100);
    if (amount <= 0) return;
    lp->enabled = 1;
    lp->amount = amount;
    lp->alpha_q15 = 24576 - (amount * 220);
    lp->alpha_q15 = dsp_clampi(lp->alpha_q15, 2048, 24576);
}

int audio_fx_lowpass_process(AudioLowpass *lp, int input)
{
    if (!lp || !lp->enabled) return input;
    lp->state += ((input - lp->state) * lp->alpha_q15) / FX_Q15_ONE;
    return lp->state;
}

void audio_fx_moog_ladder_init(AudioMoogLadder *m, int amount, int cutoff, int resonance)
{
    if (!m) return;
    memset(m, 0, sizeof(*m));
    amount = dsp_clampi(amount, 0, 100);
    if (amount <= 0) return;
    cutoff = dsp_clampi(cutoff, 1, 100);
    resonance = dsp_clampi(resonance, 0, 100);

    m->enabled = 1;
    m->amount = amount;
    m->cutoff = cutoff;
    m->resonance = resonance;
    /* alpha controls per-stage one-pole cutoff: bigger -> brighter.
     * Curve chosen so cutoff=1 is very dark and cutoff=100 is wide open. */
    m->alpha_q15 = dsp_clampi(160 + cutoff * 280, 160, 28800);
    /* resonance feedback: at 100, k approaches 4 in Q15 (~131068).
     * Capped just below to keep self-oscillation under control. */
    m->feedback_q15 = (resonance * 122000) / 100;
}

static int moog_soft_clip(int x)
{
    /* Cheap odd-symmetric polynomial soft-clip approximating tanh for the
     * resonance feedback path. Identity around zero; bends into saturation
     * above FX_DRIVE_SOFT_CLIP, hard-limited to FX_BUFFER_CLAMP. */
    int sign = 1;
    int a;
    int clipped;
    if (x < 0) { sign = -1; x = -x; }
    a = x;
    if (a <= FX_DRIVE_SOFT_CLIP) return sign * a;
    clipped = FX_DRIVE_SOFT_CLIP + ((a - FX_DRIVE_SOFT_CLIP) * 384)
              / ((a - FX_DRIVE_SOFT_CLIP) + 384);
    return clamp_sample(sign * clipped, FX_BUFFER_CLAMP);
}

int audio_fx_moog_ladder_process(AudioMoogLadder *m, int input)
{
    int driven;
    int wet;
    int dry;
    if (!m || !m->enabled) return input;

    /* Feedback path: subtract a soft-clipped resonant tap from the input
     * before the ladder. Without saturation, high-Q would blow up Q15. */
    driven = input - ((m->stage[3] * m->feedback_q15) >> 15);
    driven = moog_soft_clip(driven);

    /* Four cascaded one-pole lowpass stages. */
    m->stage[0] += (m->alpha_q15 * (driven      - m->stage[0])) >> 15;
    m->stage[1] += (m->alpha_q15 * (m->stage[0] - m->stage[1])) >> 15;
    m->stage[2] += (m->alpha_q15 * (m->stage[1] - m->stage[2])) >> 15;
    m->stage[3] += (m->alpha_q15 * (m->stage[2] - m->stage[3])) >> 15;

    m->stage[0] = clamp_sample(m->stage[0], FX_BUFFER_CLAMP);
    m->stage[1] = clamp_sample(m->stage[1], FX_BUFFER_CLAMP);
    m->stage[2] = clamp_sample(m->stage[2], FX_BUFFER_CLAMP);
    m->stage[3] = clamp_sample(m->stage[3], FX_BUFFER_CLAMP);

    wet = (m->stage[3] * m->amount) / 100;
    dry = (input * (100 - m->amount)) / 100;
    return dry + wet;
}

void audio_fx_sidechain_init(AudioSidechain *sc, int sample_rate, int amount, int release_ms)
{
    int release_samples;
    if (!sc) return;
    memset(sc, 0, sizeof(*sc));
    amount = dsp_clampi(amount, 0, 100);
    if (amount <= 0) {
        sc->env_q15 = FX_Q15_ONE;
        return;
    }
    sc->enabled = 1;
    sc->amount = amount;
    sc->release_ms = dsp_clampi(release_ms, 10, 2000);
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
    state->mix_percent = dsp_clampi(bus->mix_percent, 0, 100);
    state->drive_amount = dsp_clampi(bus->drive_amount, 0, 100);
    audio_fx_lowpass_init(&state->lowpass, bus->lowpass_amount);
    audio_fx_moog_ladder_init(&state->ladder, bus->ladder_amount, bus->ladder_cutoff, bus->ladder_resonance);
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
    sample = audio_fx_moog_ladder_process(&state->ladder, sample);
    sample = audio_fx_delay_process(&state->delay, sample);
    sample = audio_fx_sidechain_process(&state->sidechain, sample, trigger, &env_q15);
    sample = (sample * state->mix_percent) / 100;

    if (out_sidechain_env_q15) *out_sidechain_env_q15 = env_q15;
    return sample;
}

void audio_fx_clip_stats_reset(AudioClipStats *stats)
{
    if (!stats) return;
    memset(stats, 0, sizeof(*stats));
}

void audio_fx_clip_stats_push(AudioClipStats *stats, unsigned char sample)
{
    if (!stats) return;
    if (sample == 0 || sample == 255) {
        stats->edge_run++;
        if (stats->edge_run >= 4)
            stats->clipping_count++;
        return;
    }
    stats->edge_run = 0;
}
