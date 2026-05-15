#include "audio_dsp.h"

#include <limits.h>
#include <string.h>
#include <math.h>

#define DSP_PHASE_SCALE_U32 4294967296.0
#define DSP_PHASE_MAX_U32   ((double)UINT32_MAX)
#define DSP_TRI_RAMP_MASK   0x7fffu
#if defined(CLOCK_MONOTONIC)
#define DSP_PROFILE_TICKS_ARE_NS 1
#endif

static uint32_t u32_from_fraction(double x)
{
    if (x <= 0.0) return 0;
    if (x >= 1.0) return UINT32_MAX;
    return (uint32_t)(x * DSP_PHASE_MAX_U32);
}

int dsp_clampi(int value, int lo, int hi)
{
    if (value < lo) return lo;
    if (value > hi) return hi;
    return value;
}

void dsp_osc_init(DspOscillator *osc, DspWaveform waveform, int amplitude)
{
    memset(osc, 0, sizeof(*osc));
    osc->waveform = waveform;
    osc->amplitude = amplitude;
    osc->pulse_width = 0x80000000u;
    osc->noise_state = 0x6d2b79f5u;
}

void dsp_osc_set_frequency(DspOscillator *osc, double freq_hz, int sample_rate)
{
    if (freq_hz <= 0.0 || sample_rate <= 0) {
        osc->increment = 0;
        return;
    }
    osc->increment = (uint32_t)((freq_hz * DSP_PHASE_SCALE_U32) / (double)sample_rate);
    if (osc->increment == 0) osc->increment = 1;
}

void dsp_osc_set_pulse_width_percent(DspOscillator *osc, int duty_percent)
{
    if (duty_percent < 1) duty_percent = 1;
    if (duty_percent > 99) duty_percent = 99;
    osc->pulse_width = u32_from_fraction((double)duty_percent / 100.0);
}

int dsp_osc_next(DspOscillator *osc)
{
    int sample = 0;
    uint32_t phase = osc->phase;

    switch (osc->waveform) {
        case DSP_WAVE_SQUARE:
            sample = (phase < 0x80000000u) ? osc->amplitude : -osc->amplitude;
            break;
        case DSP_WAVE_PULSE:
            sample = (phase < osc->pulse_width) ? osc->amplitude : -osc->amplitude;
            break;
        case DSP_WAVE_TRIANGLE: {
            uint32_t p = phase >> 16;
            int tri = (phase & 0x80000000u)
                ? (int)(65535u - ((p & DSP_TRI_RAMP_MASK) << 1))
                : (int)((p & DSP_TRI_RAMP_MASK) << 1);
            sample = ((tri - 32768) * osc->amplitude) / 32768;
            break;
        }
        case DSP_WAVE_NOISE:
            osc->noise_state ^= osc->noise_state << 13;
            osc->noise_state ^= osc->noise_state >> 17;
            osc->noise_state ^= osc->noise_state << 5;
            sample = (osc->noise_state & 1u) ? osc->amplitude : -osc->amplitude;
            break;
    }

    osc->phase += osc->increment;
    return sample;
}

void dsp_envelope_init(DspEnvelopeRuntime *runtime, const DspEnvelope *env,
                       int sample_rate, int gate_samples)
{
    if (!runtime) return;
    memset(runtime, 0, sizeof(*runtime));
    runtime->sample_rate = sample_rate;
    if (env) runtime->env = *env;
    runtime->env.sustain_level = dsp_clampi(runtime->env.sustain_level, 0, 100);
    runtime->env.gate_percent = dsp_clampi(runtime->env.gate_percent, 1, 100);
    runtime->attack_samples = dsp_samples_from_ms(sample_rate, runtime->env.attack_ms);
    runtime->decay_samples = dsp_samples_from_ms(sample_rate, runtime->env.decay_ms);
    runtime->release_samples = dsp_samples_from_ms(sample_rate, runtime->env.release_ms);
    runtime->gate_samples = gate_samples > 0 ? gate_samples : 1;
    runtime->phase = DSP_ENV_ATTACK;
}

void dsp_envelope_note_off(DspEnvelopeRuntime *runtime)
{
    if (!runtime || runtime->phase == DSP_ENV_IDLE || runtime->phase == DSP_ENV_RELEASE)
        return;
    runtime->phase = DSP_ENV_RELEASE;
    runtime->phase_progress = 0;
    runtime->release_start_q15 = runtime->level_q15;
}

int dsp_envelope_next_q15(DspEnvelopeRuntime *runtime)
{
    int sustain_q15;
    if (!runtime) return 0;
    sustain_q15 = (runtime->env.sustain_level * 32767) / 100;

    if (runtime->phase != DSP_ENV_IDLE) {
        runtime->gate_progress++;
        if (runtime->gate_progress >= runtime->gate_samples)
            dsp_envelope_note_off(runtime);
    }

    switch (runtime->phase) {
        case DSP_ENV_IDLE:
            runtime->level_q15 = 0;
            break;
        case DSP_ENV_ATTACK:
            if (runtime->attack_samples <= 0) {
                runtime->level_q15 = 32767;
                runtime->phase = DSP_ENV_DECAY;
                runtime->phase_progress = 0;
            } else {
                runtime->phase_progress++;
                runtime->level_q15 = (runtime->phase_progress * 32767) / runtime->attack_samples;
                if (runtime->phase_progress >= runtime->attack_samples) {
                    runtime->level_q15 = 32767;
                    runtime->phase = DSP_ENV_DECAY;
                    runtime->phase_progress = 0;
                }
            }
            break;
        case DSP_ENV_DECAY:
            if (runtime->decay_samples <= 0) {
                runtime->level_q15 = sustain_q15;
                runtime->phase = DSP_ENV_SUSTAIN;
                runtime->phase_progress = 0;
            } else {
                int delta = 32767 - sustain_q15;
                runtime->phase_progress++;
                runtime->level_q15 = 32767 - (runtime->phase_progress * delta) / runtime->decay_samples;
                if (runtime->phase_progress >= runtime->decay_samples) {
                    runtime->level_q15 = sustain_q15;
                    runtime->phase = DSP_ENV_SUSTAIN;
                    runtime->phase_progress = 0;
                }
            }
            break;
        case DSP_ENV_SUSTAIN:
            runtime->level_q15 = sustain_q15;
            break;
        case DSP_ENV_RELEASE:
            if (runtime->release_samples <= 0) {
                runtime->level_q15 = 0;
                runtime->phase = DSP_ENV_IDLE;
                runtime->phase_progress = 0;
            } else {
                runtime->phase_progress++;
                runtime->level_q15 = runtime->release_start_q15 -
                    (runtime->phase_progress * runtime->release_start_q15) / runtime->release_samples;
                if (runtime->phase_progress >= runtime->release_samples) {
                    runtime->level_q15 = 0;
                    runtime->phase = DSP_ENV_IDLE;
                    runtime->phase_progress = 0;
                }
            }
            break;
    }

    runtime->level_q15 = dsp_clampi(runtime->level_q15, 0, 32767);
    return runtime->level_q15;
}

int dsp_tri_lfo_q15(uint32_t *phase, int rate_millihz, int sample_rate)
{
    uint64_t increment;
    uint32_t p;
    int tri;

    if (!phase || rate_millihz <= 0 || sample_rate <= 0) return 0;
    increment = ((uint64_t)rate_millihz << 32) / ((uint64_t)sample_rate * 1000ull);
    *phase += (uint32_t)increment;
    p = *phase >> 16;
    tri = (*phase & 0x80000000u)
        ? (int)(65535u - ((p & 0x7fffu) << 1))
        : (int)((p & 0x7fffu) << 1);
    return tri - 32768;
}

double dsp_freq_with_cents(double freq, int cents)
{
    if (freq <= 0.0 || cents == 0) return freq;
    return freq * pow(2.0, (double)cents / 1200.0);
}

void dsp_stepper_init(DspSampleStepper *stepper, int sample_rate, int step_ms)
{
    uint64_t n = (uint64_t)(sample_rate > 0 ? sample_rate : 0) * (uint64_t)(step_ms > 0 ? step_ms : 0);
    stepper->numer = (uint32_t)n;
    stepper->denom = 1000u;
    stepper->rem = 0u;
}

int dsp_stepper_next(DspSampleStepper *stepper)
{
    uint64_t sum = (uint64_t)stepper->numer + stepper->rem;
    int samples = (int)(sum / stepper->denom);
    stepper->rem = (uint32_t)(sum % stepper->denom);
    return samples;
}

int dsp_total_samples_for_steps(int sample_rate, int step_ms, int steps)
{
    if (sample_rate <= 0 || step_ms <= 0 || steps <= 0) return 0;
    return (int)(((uint64_t)sample_rate * (uint64_t)step_ms * (uint64_t)steps) / 1000u);
}

int dsp_samples_from_ms(int sample_rate, int ms)
{
    if (sample_rate <= 0 || ms <= 0) return 0;
    return (int)(((uint64_t)sample_rate * (uint64_t)ms) / 1000u);
}

int dsp_clamp_u8(int value)
{
    if (value < 0) return 0;
    if (value > 255) return 255;
    return value;
}

void dsp_profile_reset(DspProfile *p)
{
    memset(p, 0, sizeof(*p));
}

void dsp_profile_add_generation(DspProfile *p, int samples, uint64_t ticks)
{
    if (!p) return;
    p->generation_calls++;
    if (samples > 0) p->generated_samples += (uint64_t)samples;
    p->generation_ticks += ticks;
}

void dsp_profile_add_write(DspProfile *p, int bytes, int requested, int is_error)
{
    if (!p) return;
    p->write_calls++;
    if (bytes > 0) p->write_bytes += (uint64_t)bytes;
    if (is_error) {
        p->write_errors++;
        p->underruns++;
        return;
    }
    if (bytes >= 0 && requested > bytes) p->write_short++;
}

uint64_t dsp_profile_now_ticks(void)
{
#if defined(CLOCK_MONOTONIC)
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) == 0)
        return ((uint64_t)ts.tv_sec * 1000000000ull) + (uint64_t)ts.tv_nsec;
#endif
    return (uint64_t)clock();
}

uint64_t dsp_profile_ticks_to_ns(uint64_t ticks)
{
#if defined(DSP_PROFILE_TICKS_ARE_NS)
    return ticks;
#else
    if (CLOCKS_PER_SEC <= 0) return 0;
    return (ticks * 1000000000ull) / (uint64_t)CLOCKS_PER_SEC;
#endif
}
