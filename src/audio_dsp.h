#ifndef MEMDECK_AUDIO_DSP_H
#define MEMDECK_AUDIO_DSP_H

#include <stdint.h>
#include <time.h>

#define DSP_PHASE_BITS 32u

typedef enum {
    DSP_WAVE_SQUARE = 0,
    DSP_WAVE_PULSE,
    DSP_WAVE_TRIANGLE,
    DSP_WAVE_NOISE
} DspWaveform;

typedef struct {
    uint32_t phase;
    uint32_t increment;
    uint32_t pulse_width; /* 0..UINT32_MAX threshold */
    uint32_t noise_state;
    int amplitude;
    DspWaveform waveform;
} DspOscillator;

typedef struct {
    uint32_t numer;
    uint32_t denom;
    uint32_t rem;
} DspSampleStepper;

typedef struct {
    uint64_t generated_samples;
    uint64_t generation_calls;
    uint64_t generation_ticks;
    uint64_t write_calls;
    uint64_t write_bytes;
    uint64_t write_short;
    uint64_t write_errors;
    uint64_t underruns;
} DspProfile;

void dsp_osc_init(DspOscillator *osc, DspWaveform waveform, int amplitude);
void dsp_osc_set_frequency(DspOscillator *osc, double freq_hz, int sample_rate);
void dsp_osc_set_pulse_width_percent(DspOscillator *osc, int duty_percent);
int  dsp_osc_next(DspOscillator *osc);

void dsp_stepper_init(DspSampleStepper *stepper, int sample_rate, int step_ms);
int  dsp_stepper_next(DspSampleStepper *stepper);
int  dsp_total_samples_for_steps(int sample_rate, int step_ms, int steps);
int  dsp_samples_from_ms(int sample_rate, int ms);

int  dsp_clamp_u8(int value);

void dsp_profile_reset(DspProfile *p);
void dsp_profile_add_generation(DspProfile *p, int samples, uint64_t ticks);
void dsp_profile_add_write(DspProfile *p, int bytes, int requested, int is_error);
uint64_t dsp_profile_now_ticks(void);
uint64_t dsp_profile_ticks_to_ns(uint64_t ticks);

#endif
