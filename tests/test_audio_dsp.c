/*
 * Deterministic regression tests for src/audio_dsp.c.
 *
 * Each test produces a pass/fail result based on exact integer arithmetic so
 * the expected values are reproducible across platforms and compiler versions.
 *
 * Build (via Makefile target "test-audio"):
 *   $(CC) $(CFLAGS) -o bin/test-audio-dsp tests/test_audio_dsp.c src/audio_dsp.c $(LDFLAGS)
 */

#include "../src/audio_dsp.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

static int g_fails = 0;

static void check_int(const char *name, int got, int expected)
{
    if (got != expected) {
        printf("FAIL %s: got %d, expected %d\n", name, got, expected);
        g_fails++;
    }
}

static void check_u32(const char *name, uint32_t got, uint32_t expected)
{
    if (got != expected) {
        printf("FAIL %s: got %u, expected %u\n", name, got, expected);
        g_fails++;
    }
}

/* ─── 1. dsp_clamp_u8 ────────────────────────────────────────── */

static void test_clamp(void)
{
    check_int("clamp(-1)",   dsp_clamp_u8(-1),   0);
    check_int("clamp(0)",    dsp_clamp_u8(0),     0);
    check_int("clamp(1)",    dsp_clamp_u8(1),     1);
    check_int("clamp(128)",  dsp_clamp_u8(128),   128);
    check_int("clamp(255)",  dsp_clamp_u8(255),   255);
    check_int("clamp(256)",  dsp_clamp_u8(256),   255);
    check_int("clamp(1000)", dsp_clamp_u8(1000),  255);
}

/* ─── 2. dsp_samples_from_ms ─────────────────────────────────── */

static void test_samples_from_ms(void)
{
    /* exact integer ms → sample counts */
    check_int("sfm(22050,0)",     dsp_samples_from_ms(22050,    0), 0);
    check_int("sfm(22050,1000)",  dsp_samples_from_ms(22050, 1000), 22050);
    check_int("sfm(22050,500)",   dsp_samples_from_ms(22050,  500), 11025);
    /* 22050 * 125 / 1000 = 2756 (truncated) */
    check_int("sfm(22050,125)",   dsp_samples_from_ms(22050,  125), 2756);
    /* edge: zero sample rate */
    check_int("sfm(0,1000)",      dsp_samples_from_ms(0,     1000), 0);
}

/* ─── 3. dsp_total_samples_for_steps ────────────────────────── */

static void test_total_samples(void)
{
    /* 22050 * 125 * 64 / 1000 = 176400 */
    check_int("total(22050,125,64)", dsp_total_samples_for_steps(22050, 125, 64), 176400);
    check_int("total(22050,125,1)",  dsp_total_samples_for_steps(22050, 125,  1),   2756);
    check_int("total(22050,125,0)",  dsp_total_samples_for_steps(22050, 125,  0),      0);
    check_int("total(0,125,64)",     dsp_total_samples_for_steps(0,     125, 64),      0);
}

#define SAMPLE_RATE 22050

/* ─── 4. dsp_osc_set_frequency — zero-frequency edge case ───── */

static void test_osc_frequency(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_SQUARE, 64);
    dsp_osc_set_frequency(&osc, 0.0, SAMPLE_RATE);
    check_u32("inc_zero_freq", osc.increment, 0);
}

/* ─── 5. Square wave — half-period transition ────────────────── */

/*
 * The 440 Hz oscillator at 22050 Hz crosses the 0x80000000 phase threshold
 * (half-period) between sample 25 and sample 26 regardless of which side of
 * the floating-point rounding the increment falls:
 *
 *   increment ≈ 85704562  (actual platform value)
 *   half-period threshold = 0x80000000 = 2147483648
 *   N=25: 25 * increment = 2142614050 < 2147483648  → +amplitude
 *   N=26: 26 * increment = 2228318612 > 2147483648  → −amplitude
 */
static void test_square_wave(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_SQUARE, 100);
    dsp_osc_set_frequency(&osc, 440.0, SAMPLE_RATE);

    /* Positive half: samples 0 – 25 */
    for (int i = 0; i <= 25; i++) {
        int s = dsp_osc_next(&osc);
        if (s != 100) {
            printf("FAIL square_pos[%d]: expected 100, got %d\n", i, s);
            g_fails++;
        }
    }
    /* First negative half: sample 26 */
    check_int("square_neg_26", dsp_osc_next(&osc), -100);
}

/* ─── 6. Pulse wave — 25% duty cycle ────────────────────────── */

/*
 * pulse_width = u32_from_fraction(0.25) ≈ 1073741823
 *
 *   N=12: 12 * increment ≈ 1028454744 < 1073741823 → +amplitude
 *   N=13: 13 * increment ≈ 1114159306 > 1073741823 → −amplitude
 */
static void test_pulse_wave(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_PULSE, 80);
    dsp_osc_set_frequency(&osc, 440.0, SAMPLE_RATE);
    dsp_osc_set_pulse_width_percent(&osc, 25);

    for (int i = 0; i <= 12; i++) {
        int s = dsp_osc_next(&osc);
        if (s != 80) {
            printf("FAIL pulse_pos[%d]: expected 80, got %d\n", i, s);
            g_fails++;
        }
    }
    check_int("pulse_neg_13", dsp_osc_next(&osc), -80);
}

/* ─── 7. Triangle wave — value at phase 0 ───────────────────── */

/*
 * At phase=0:
 *   p = 0 >> 16 = 0
 *   (phase & 0x80000000) = 0  → tri = (0 & 0x7fff) << 1 = 0
 *   sample = ((0 - 32768) * amplitude) / 32768
 *
 * With amplitude=32768: sample = (-32768 * 32768) / 32768 = -32768
 */
static void test_triangle_wave(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_TRIANGLE, 32768);
    /* increment=0 keeps phase at 0 for deterministic check */
    check_int("triangle_phase0", dsp_osc_next(&osc), -32768);
}

/* ─── 8. Noise wave — LFSR produces non-constant output ──────── */

static void test_noise_wave(void)
{
    DspOscillator osc;
    dsp_osc_init(&osc, DSP_WAVE_NOISE, 64);
    dsp_osc_set_frequency(&osc, 440.0, SAMPLE_RATE);

    int first = dsp_osc_next(&osc);
    int all_same = 1;
    for (int i = 0; i < 63; i++) {
        if (dsp_osc_next(&osc) != first) {
            all_same = 0;
            break;
        }
    }
    if (all_same) {
        printf("FAIL noise_varies: all 64 samples were %d (LFSR stuck?)\n", first);
        g_fails++;
    }
}

/* ─── 9. DspSampleStepper — deterministic distribution ──────── */

/*
 * dsp_stepper_init(&s, 22050, 125):
 *   numer = 22050 * 125 = 2756250, denom = 1000, rem = 0
 *
 * Call 1: sum = 2756250+0   = 2756250, samples = 2756, rem = 250
 * Call 2: sum = 2756250+250 = 2756500, samples = 2756, rem = 500
 * Call 3: sum = 2756250+500 = 2756750, samples = 2756, rem = 750
 * Call 4: sum = 2756250+750 = 2757000, samples = 2757, rem = 0
 * Call 5: same as call 1 → 2756
 */
static void test_stepper(void)
{
    DspSampleStepper s;
    dsp_stepper_init(&s, SAMPLE_RATE, 125);
    check_int("stepper_1", dsp_stepper_next(&s), 2756);
    check_int("stepper_2", dsp_stepper_next(&s), 2756);
    check_int("stepper_3", dsp_stepper_next(&s), 2756);
    check_int("stepper_4", dsp_stepper_next(&s), 2757);
    check_int("stepper_5", dsp_stepper_next(&s), 2756); /* cycle restarts */
}

/* ─── 10. Stepper sum matches total_samples_for_steps ───────── */

static void test_stepper_sum(void)
{
    DspSampleStepper s;
    dsp_stepper_init(&s, SAMPLE_RATE, 125);
    int total = 0;
    for (int i = 0; i < 64; i++)
        total += dsp_stepper_next(&s);
    check_int("stepper_sum64", total, dsp_total_samples_for_steps(SAMPLE_RATE, 125, 64));
}

/* ─── 11. ADSR progression + gate/release ────────────────────── */

static void test_adsr_progression(void)
{
    DspEnvelope env = { 10, 10, 50, 10, 50 };
    DspEnvelopeRuntime rt;
    int attack_samples = dsp_samples_from_ms(SAMPLE_RATE, env.attack_ms);
    int decay_samples = dsp_samples_from_ms(SAMPLE_RATE, env.decay_ms);
    int release_samples = dsp_samples_from_ms(SAMPLE_RATE, env.release_ms);
    int gate_samples = dsp_samples_from_ms(SAMPLE_RATE, 40);
    int v = 0;

    dsp_envelope_init(&rt, &env, SAMPLE_RATE, gate_samples);
    check_int("adsr_attack_phase_start", rt.phase, DSP_ENV_ATTACK);

    for (int i = 0; i < attack_samples; i++)
        v = dsp_envelope_next_q15(&rt);
    check_int("adsr_attack_peak", v, 32767);
    check_int("adsr_decay_phase_after_attack", rt.phase, DSP_ENV_DECAY);

    for (int i = 0; i < decay_samples; i++)
        v = dsp_envelope_next_q15(&rt);
    check_int("adsr_sustain_level", v, (50 * 32767) / 100);

    for (int i = attack_samples + decay_samples; i < gate_samples + release_samples + 4; i++)
        v = dsp_envelope_next_q15(&rt);
    check_int("adsr_released_to_zero", v, 0);
    check_int("adsr_idle_after_release", rt.phase, DSP_ENV_IDLE);
}

static void test_adsr_note_off(void)
{
    DspEnvelope env = { 0, 0, 80, 10, 100 };
    DspEnvelopeRuntime rt;
    int release_samples = dsp_samples_from_ms(SAMPLE_RATE, env.release_ms);
    int v = 0;

    dsp_envelope_init(&rt, &env, SAMPLE_RATE, dsp_samples_from_ms(SAMPLE_RATE, 200));
    for (int i = 0; i < 16; i++)
        v = dsp_envelope_next_q15(&rt);
    check_int("adsr_manual_off_pre_level", v, (80 * 32767) / 100);
    dsp_envelope_note_off(&rt);
    check_int("adsr_manual_off_phase", rt.phase, DSP_ENV_RELEASE);
    for (int i = 0; i < release_samples + 2; i++)
        v = dsp_envelope_next_q15(&rt);
    check_int("adsr_manual_off_zero", v, 0);
}

/* ─── main ───────────────────────────────────────────────────── */

int main(void)
{
    printf("=== Audio DSP regression tests ===\n");

    test_clamp();
    test_samples_from_ms();
    test_total_samples();
    test_osc_frequency();
    test_square_wave();
    test_pulse_wave();
    test_triangle_wave();
    test_noise_wave();
    test_stepper();
    test_stepper_sum();
    test_adsr_progression();
    test_adsr_note_off();

    printf("Audio DSP regression: %d failure(s)\n", g_fails);
    printf("%s\n", g_fails == 0 ? "All tests passed." : "Some tests FAILED.");
    return g_fails > 0 ? 1 : 0;
}
