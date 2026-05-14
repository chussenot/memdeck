#include "../src/audio_dsp.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SAMPLE_RATE 22050
#define ITERATIONS 2000
#define STEP_MS 125
#define STEPS 64

static uint64_t run_osc_bench(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_SQUARE, 64);
    dsp_osc_set_frequency(&osc, 440.0, SAMPLE_RATE);

    uint64_t t0 = dsp_profile_now_ticks();
    volatile int sink = 0;
    for (int j = 0; j < ITERATIONS; j++) {
        for (int i = 0; i < SAMPLE_RATE; i++)
            sink += dsp_osc_next(&osc);
    }
    uint64_t dt = dsp_profile_now_ticks() - t0;
    (void)sink;
    return dt;
}

static uint64_t run_mix_bench(void)
{
    int total = dsp_total_samples_for_steps(SAMPLE_RATE, STEP_MS, STEPS);
    unsigned char *buf = malloc((size_t)total);
    if (!buf) return 0;

    uint64_t t0 = dsp_profile_now_ticks();
    for (int it = 0; it < ITERATIONS; it++) {
        memset(buf, 128, (size_t)total);
        DspSampleStepper stepper;
        dsp_stepper_init(&stepper, SAMPLE_RATE, STEP_MS);
        int base = 0;

        for (int step = 0; step < STEPS; step++) {
            int step_samples = dsp_stepper_next(&stepper);
            DspOscillator bass, arp, lead;
            dsp_osc_init(&bass, DSP_WAVE_SQUARE, 45);
            dsp_osc_set_frequency(&bass, 110.0, SAMPLE_RATE);
            dsp_osc_init(&arp, DSP_WAVE_PULSE, 28);
            dsp_osc_set_frequency(&arp, 440.0, SAMPLE_RATE);
            dsp_osc_set_pulse_width_percent(&arp, 25);
            dsp_osc_init(&lead, DSP_WAVE_SQUARE, 40);
            dsp_osc_set_frequency(&lead, 659.25, SAMPLE_RATE);

            int arp_end = dsp_samples_from_ms(SAMPLE_RATE, (STEP_MS * 3) / 4);
            int lead_end = (step_samples * 9) / 10;

            for (int i = 0; i < step_samples; i++) {
                int val = 128 + dsp_osc_next(&bass);
                if (i < arp_end) val += dsp_osc_next(&arp);
                if (i < lead_end) val += dsp_osc_next(&lead);
                buf[base + i] = (unsigned char)dsp_clamp_u8(val);
            }
            base += step_samples;
        }
    }

    uint64_t dt = dsp_profile_now_ticks() - t0;
    free(buf);
    return dt;
}

static uint64_t run_stepper_bench(void)
{
    uint64_t t0 = dsp_profile_now_ticks();
    volatile int sum = 0;
    for (int j = 0; j < ITERATIONS * 1000; j++) {
        DspSampleStepper stepper;
        dsp_stepper_init(&stepper, SAMPLE_RATE, STEP_MS);
        for (int i = 0; i < STEPS; i++)
            sum += dsp_stepper_next(&stepper);
    }
    uint64_t dt = dsp_profile_now_ticks() - t0;
    (void)sum;
    return dt;
}

int main(void)
{
    uint64_t osc_ticks = run_osc_bench();
    uint64_t mix_ticks = run_mix_bench();
    uint64_t step_ticks = run_stepper_bench();

    printf("Audio microbenchmark (ticks + ns)\n");
    printf("oscillator: %llu ticks (%llu ns)\n",
           (unsigned long long)osc_ticks,
           (unsigned long long)dsp_profile_ticks_to_ns(osc_ticks));
    printf("mix loop:   %llu ticks (%llu ns)\n",
           (unsigned long long)mix_ticks,
           (unsigned long long)dsp_profile_ticks_to_ns(mix_ticks));
    printf("stepper:    %llu ticks (%llu ns)\n",
           (unsigned long long)step_ticks,
           (unsigned long long)dsp_profile_ticks_to_ns(step_ticks));

    return 0;
}
