/*
 * Verify ABC parser for both menu tracks.
 * Build (via Makefile target "test-abc"):
 *   $(CC) $(CFLAGS) -o bin/test-abc tests/test_abc.c src/abc.c src/card.c src/audio_dsp.c $(LDFLAGS)
 */
#include <stdio.h>
#include <math.h>
#include <stdint.h>
#include "../src/memdeck.h"

static int g_failures = 0;

static uint64_t fnv1a64(const unsigned char *data, int len)
{
    uint64_t hash = 1469598103934665603ull;
    for (int i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= 1099511628211ull;
    }
    return hash;
}

static void failf(const char *label, const char *msg)
{
    printf("FAIL %s: %s\n", label, msg);
    g_failures++;
}

static int test_track(const char *bass, const char *arp, const char *lead,
                      const char *label, int expect_steps)
{
    const char *paths[3] = { bass, arp, lead };
    AbcMusic music;

    if (abc_load_voices(paths, 3, &music) != 0) {
        printf("FAIL %s: could not load ABC files\n", label);
        return 0;
    }

    printf("[%s] \"%s\" — %d voices, BPM=%d, step_ms=%d\n",
           label, music.title, music.voice_count, music.bpm, music.step_ms);

    int ok = 1;
    for (int v = 0; v < music.voice_count; v++) {
        AbcVoice *voice = &music.voices[v];
        printf("  Voice %d (%s): %d notes, amp=%d%s\n",
               v, voice->name, voice->note_count, voice->amplitude,
               voice->staccato ? " staccato" : "");
        if (expect_steps > 0 && voice->note_count != expect_steps) {
            printf("    FAIL: expected %d notes\n", expect_steps);
            ok = 0;
        }
    }

    int pcm_len = 0;
    unsigned char *pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len == 0) {
        printf("  FAIL: PCM generation failed\n");
        ok = 0;
    } else {
        printf("  PCM: %d samples (%.2f seconds)\n", pcm_len, pcm_len / 22050.0);
        free(pcm);
    }

    return ok;
}

static void test_golden_fixture(void)
{
    static const unsigned char expected_prefix[] = {
        188, 188, 188, 188, 188, 188, 188, 188,
        188, 188, 188, 188, 188, 188, 188, 188,
        188, 140, 140, 140, 140, 140, 140, 140,
        140, 140, 140, 140, 140, 140, 140, 140
    };
    static const uint64_t expected_checksum = 0xa8f7ce1aa65292bbull;
    const char *path = "tests/fixtures/golden_small.abc";
    AbcMusic music;
    int pcm_len = 0;
    unsigned char *pcm = NULL;

    if (abc_load(path, &music) != 0) {
        failf("golden fixture", "could not load tests/fixtures/golden_small.abc");
        return;
    }

    pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm) {
        failf("golden fixture", "PCM generation returned NULL");
        return;
    }

    if (music.voice_count != 2) {
        printf("FAIL golden fixture: got %d voices, expected 2\n", music.voice_count);
        g_failures++;
    }
    if (pcm_len != 44100) {
        printf("FAIL golden fixture: got PCM len %d, expected 44100\n", pcm_len);
        g_failures++;
    }

    uint64_t checksum = fnv1a64(pcm, pcm_len);
    if (checksum != expected_checksum) {
        printf("FAIL golden fixture: got checksum 0x%016llx, expected 0x%016llx\n",
               (unsigned long long)checksum,
               (unsigned long long)expected_checksum);
        g_failures++;
    }

    if (memcmp(pcm, expected_prefix, sizeof(expected_prefix)) != 0) {
        printf("FAIL golden fixture: first %zu samples mismatch\n",
               sizeof(expected_prefix));
        g_failures++;
    }

    free(pcm);
}

static void test_dsl_directives(void)
{
    const char *path = "data/music/dark_moroder.abc";
    AbcMusic music;

    printf("[DSL Test] Loading %s\n", path);
    if (abc_load(path, &music) != 0) {
        failf("DSL directives", "could not load dark_moroder.abc");
        return;
    }

    /* Verify swing directive was parsed */
    if (music.swing_pct != 56) {
        printf("FAIL DSL: expected swing_pct=56, got %d\n", music.swing_pct);
        g_failures++;
    } else {
        printf("  ✓ Swing: %d%%\n", music.swing_pct);
    }

    /* Verify FX directives were parsed */
    printf("  ✓ FX delay steps: %d\n", music.fx_delay_steps);
    printf("  ✓ FX sidechain amount: %d%%\n", music.fx_sidechain_amount);
    printf("  ✓ FX sidechain release: %dms\n", music.fx_sidechain_release_ms);

    /* Verify voices parsed with instruments */
    if (music.voice_count != 3) {
        printf("FAIL DSL: expected 3 voices, got %d\n", music.voice_count);
        g_failures++;
    } else {
        printf("  ✓ Voices: %d\n", music.voice_count);
        for (int i = 0; i < music.voice_count; i++) {
            AbcVoice *v = &music.voices[i];
            printf("    Voice %d (%s): amp=%d wave=%d duty=%d attack=%d decay=%d sustain=%d release=%d\n",
                   i, v->name, v->amplitude, v->waveform, v->duty_cycle,
                   v->attack_ms, v->decay_ms, v->sustain_level, v->release_ms);
        }
    }

    /* Generate PCM to ensure rendering works */
    int pcm_len = 0;
    unsigned char *pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len == 0) {
        failf("DSL directives", "PCM generation failed");
        return;
    }
    printf("  ✓ PCM: %d samples (%.2f seconds)\n", pcm_len, pcm_len / 22050.0);
    free(pcm);
}

static void test_advanced_dsl_features(void)
{
    const char *path = "data/music/advanced_dsl_demo.abc";
    AbcMusic music;

    printf("[Advanced DSL Test] Loading %s\n", path);
    if (abc_load(path, &music) != 0) {
        failf("Advanced DSL", "could not load advanced_dsl_demo.abc");
        return;
    }

    /* Verify instruments were parsed */
    if (music.instrument_count != 4) {
        printf("FAIL Advanced DSL: expected 4 instruments, got %d\n", music.instrument_count);
        g_failures++;
    } else {
        printf("  ✓ Instruments: %d\n", music.instrument_count);
        for (int i = 0; i < music.instrument_count; i++) {
            AbcInstrument *inst = &music.instruments[i];
            printf("    Instrument %d (%s): preset=%s amp=%d wave=%d fx=%d\n",
                   i, inst->name, inst->preset, inst->amplitude, inst->waveform, inst->fx_bus);
        }
    }

    /* Verify FX buses were parsed */
    if (music.fx_bus_count < 2) {
        printf("FAIL Advanced DSL: expected at least 2 FX buses, got %d\n", music.fx_bus_count);
        g_failures++;
    } else {
        printf("  ✓ FX Buses: %d\n", music.fx_bus_count);
        for (int i = 0; i < music.fx_bus_count && i < 2; i++) {
            AbcFxBus *bus = &music.fx_buses[i];
            printf("    Bus %d: delay=%d drive=%d lowpass=%d mix=%d\n",
                   i, bus->delay_steps, bus->drive_amount, bus->lowpass_amount, bus->mix_percent);
        }
    }

    /* Verify voices reference instruments */
    printf("  ✓ Voice-Instrument mapping:\n");
    for (int i = 0; i < music.voice_count; i++) {
        AbcVoice *v = &music.voices[i];
        printf("    Voice %d (%s): instrument_ref='%s' fx_bus=%d amp=%d\n",
               i, v->name, v->instrument_ref, v->fx_bus, v->amplitude);
    }

    /* Generate PCM to ensure rendering works */
    int pcm_len = 0;
    unsigned char *pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len == 0) {
        failf("Advanced DSL", "PCM generation failed");
        return;
    }
    printf("  ✓ PCM: %d samples (%.2f seconds)\n", pcm_len, pcm_len / 22050.0);
    free(pcm);
}

static void test_multi_fx_buses(void)
{
    const char *path = "data/music/multi_fx_demo.abc";
    AbcMusic music;

    printf("[Multi-FX Test] Loading %s\n", path);
    if (abc_load(path, &music) != 0) {
        failf("Multi-FX", "could not load multi_fx_demo.abc");
        return;
    }

    /* Verify 2 FX buses configured */
    if (music.fx_bus_count < 2) {
        printf("FAIL Multi-FX: expected at least 2 FX buses, got %d\n", music.fx_bus_count);
        g_failures++;
    } else {
        printf("  ✓ FX Buses: %d\n", music.fx_bus_count);
        
        /* Bus 0: dry/punchy */
        if (music.fx_buses[0].delay_steps != 2 || music.fx_buses[0].mix_percent != 90) {
            printf("FAIL Multi-FX: bus 0 configuration incorrect\n");
            g_failures++;
        } else {
            printf("    Bus 0: delay=%d mix=%d (dry/punchy) ✓\n",
                   music.fx_buses[0].delay_steps, music.fx_buses[0].mix_percent);
        }
        
        /* Bus 1: wet/ambient */
        if (music.fx_buses[1].delay_steps != 8 || music.fx_buses[1].mix_percent != 60) {
            printf("FAIL Multi-FX: bus 1 configuration incorrect\n");
            g_failures++;
        } else {
            printf("    Bus 1: delay=%d mix=%d (wet/ambient) ✓\n",
                   music.fx_buses[1].delay_steps, music.fx_buses[1].mix_percent);
        }
    }

    /* Verify voices routed to correct FX buses */
    for (int i = 0; i < music.voice_count; i++) {
        AbcVoice *v = &music.voices[i];
        printf("    Voice %d (%s): fx_bus=%d\n", i, v->name, v->fx_bus);
    }

    /* Generate PCM */
    int pcm_len = 0;
    unsigned char *pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len == 0) {
        failf("Multi-FX", "PCM generation failed");
        return;
    }
    printf("  ✓ PCM: %d samples (%.2f seconds)\n", pcm_len, pcm_len / 22050.0);
    free(pcm);
}

int main(void)
{
    int ok = 1;

    /* Track 1: Dark Synth (D minor) — 64 steps × 2 repeat = 128 */
    ok &= test_track(
        "data/music/menu_bass.abc",
        "data/music/menu_arp.abc",
        "data/music/menu_lead.abc",
        "Track 1", 128);

    printf("\n");

    /* Track 2: C64 Nostalgia (A minor) — 64 steps × 2 repeat = 128 */
    ok &= test_track(
        "data/music/menu2_bass.abc",
        "data/music/menu2_arp.abc",
        "data/music/menu2_lead.abc",
        "Track 2", 128);

    printf("\n");
    test_golden_fixture();

    printf("\n");
    test_dsl_directives();

    printf("\n");
    test_advanced_dsl_features();

    printf("\n");
    test_multi_fx_buses();

    if (g_failures > 0) ok = 0;
    printf("\n%s\n", ok ? "All tests passed." : "Some tests FAILED.");
    return ok ? 0 : 1;
}
