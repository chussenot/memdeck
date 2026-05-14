#include "audio_dsp.h"

#include <limits.h>
#include <string.h>

#define DSP_PHASE_SCALE_U32 4294967296.0
#define DSP_PHASE_MAX_U32   ((double)UINT32_MAX)
#if defined(CLOCK_MONOTONIC)
#define DSP_PROFILE_TICKS_ARE_NS 1
#endif

static uint32_t u32_from_fraction(double x)
{
    if (x <= 0.0) return 0;
    if (x >= 1.0) return UINT32_MAX;
    return (uint32_t)(x * DSP_PHASE_MAX_U32);
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
                ? (int)(65535u - ((p & 0x7fffu) << 1))
                : (int)((p & 0x7fffu) << 1);
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
