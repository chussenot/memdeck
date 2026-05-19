/*
 * Verify ABC parser for both menu tracks.
 * Build (via Makefile target "test-abc"):
 *   $(CC) $(CFLAGS) -o bin/test-abc tests/test_abc.c src/abc.c src/card.c src/audio_dsp.c $(LDFLAGS)
 */
#include <stdio.h>
#include <math.h>
#include <stdint.h>
#include <string.h>
#include "../src/audio_engine.h"
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

static int uses_supported_directives_only(const char *path)
{
    FILE *f = fopen(path, "r");
    char line[1024];
    static const char *allowed[] = {
        "%%swing",
        "%%effect",
        "%%sidechain",
        "%%instrument",
        "%%pattern",
        "%%arrangement",
        "%%voice"
    };
    if (!f) return 0;
    while (fgets(line, sizeof(line), f)) {
        if (line[0] != '%' || line[1] != '%') continue;
        int ok = 0;
        for (size_t i = 0; i < sizeof(allowed) / sizeof(allowed[0]); i++) {
            if (strncmp(line, allowed[i], strlen(allowed[i])) == 0) {
                ok = 1;
                break;
            }
        }
        if (!ok) {
            fclose(f);
            return 0;
        }
    }
    fclose(f);
    return 1;
}

static int pcm_clipping_count(const unsigned char *pcm, int len)
{
    int c = 0;
    int run = 1;
    for (int i = 1; i < len; i++) {
        if (pcm[i] == pcm[i - 1]) run++;
        else run = 1;
        if (run >= 4 && (pcm[i] == 0 || pcm[i] == 255))
            c++;
    }
    return c;
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
        180, 180, 180, 180, 180, 180, 180, 180,
        180, 180, 180, 180, 180, 180, 180, 180,
        180, 140, 140, 140, 140, 140, 140, 140,
        140, 140, 140, 140, 140, 140, 140, 140
    };
    static const uint64_t expected_checksum = 0x0de5a3a1954d471bull;
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

static void test_showcase_demo_regression(void)
{
    typedef struct {
        const char *path;
        int expected_len;
        uint64_t expected_checksum;
        int max_clipping;
    } DemoExpectation;
    static const DemoExpectation demos[] = {
        { "data/music/dark_moroder.abc", 341419, 0x51c34a9304aa4556ull, 5000 },
        { "data/music/perturbator_loop.abc", 298140, 0xf93821e474e9e321ull, 9000 },
        { "data/music/carpenter_drive.abc", 407076, 0xaab706f01eaac6eaull, 6000 },
        { "data/music/advanced_dsl_demo.abc", 330750, 0x7bf8a698c875695dull, 2000 },
        { "data/music/multi_fx_demo.abc", 315940, 0x694dd56014426abfull, 2000 },
        { "data/music/neon_nightdrive.abc", 347016, 0xb7aa1da212da5aa9ull, 2000 },
        { "data/music/metro_chase.abc", 289972, 0x8377e2a56353b8c7ull, 16000 },
        { "data/music/black_sunrise.abc", 378000, 0x621b25ee41ae1c61ull, 2000 },
        { "data/music/machine_romance.abc", 352800, 0x1e20575de2133460ull, 5000 },
        { "data/music/hypersleep_dream.abc", 441000, 0x3bfe2cd451d41c50ull, 1000 }
    };

    for (size_t i = 0; i < sizeof(demos) / sizeof(demos[0]); i++) {
        AbcMusic music;
        unsigned char *pcm1 = NULL;
        unsigned char *pcm2 = NULL;
        AudioRenderStats stats1;
        AudioRenderStats stats2;
        int len1 = 0;
        int len2 = 0;
        uint64_t c1;
        uint64_t c2;
        int clip;

        if (!uses_supported_directives_only(demos[i].path)) {
            printf("FAIL showcase directives: %s uses unsupported %% directives\n", demos[i].path);
            g_failures++;
            continue;
        }
        if (abc_load(demos[i].path, &music) != 0) {
            printf("FAIL showcase parse: %s\n", demos[i].path);
            g_failures++;
            continue;
        }
        pcm1 = audio_engine_render_abc_file(demos[i].path, SAMPLE_RATE_ABC, &len1, &stats1);
        pcm2 = audio_engine_render_abc_file(demos[i].path, SAMPLE_RATE_ABC, &len2, &stats2);
        if (!pcm1 || !pcm2 || len1 <= 0 || len2 <= 0) {
            printf("FAIL showcase render: %s\n", demos[i].path);
            g_failures++;
            free(pcm1);
            free(pcm2);
            continue;
        }

        c1 = fnv1a64(pcm1, len1);
        c2 = fnv1a64(pcm2, len2);
        clip = pcm_clipping_count(pcm1, len1);

        if (len1 != demos[i].expected_len || len2 != demos[i].expected_len) {
            printf("FAIL showcase len: %s got %d/%d expected %d\n",
                   demos[i].path, len1, len2, demos[i].expected_len);
            g_failures++;
        }
        if (c1 != demos[i].expected_checksum || c2 != demos[i].expected_checksum || c1 != c2) {
            printf("FAIL showcase checksum: %s got 0x%016llx/0x%016llx expected 0x%016llx\n",
                   demos[i].path,
                   (unsigned long long)c1,
                   (unsigned long long)c2,
                   (unsigned long long)demos[i].expected_checksum);
            g_failures++;
        }
        if (stats1.checksum != c1 || stats2.checksum != c2 ||
            stats1.sample_count != (unsigned long long)len1 ||
            stats2.sample_count != (unsigned long long)len2) {
            printf("FAIL showcase stats: %s inconsistent render stats\n", demos[i].path);
            g_failures++;
        }
        if (clip > demos[i].max_clipping) {
            printf("FAIL showcase clipping: %s got %d max %d\n",
                    demos[i].path, clip, demos[i].max_clipping);
            g_failures++;
        }

        free(pcm1);
        free(pcm2);
    }
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
    if (music.swing_pct != 58) {
        printf("FAIL DSL: expected swing_pct=58, got %d\n", music.swing_pct);
        g_failures++;
    } else {
        printf("  ✓ Swing: %d%%\n", music.swing_pct);
    }

    /* Verify FX directives were parsed */
    printf("  ✓ FX delay steps: %d\n", music.fx_delay_steps);
    printf("  ✓ FX sidechain amount: %d%%\n", music.fx_sidechain_amount);
    printf("  ✓ FX sidechain release: %dms\n", music.fx_sidechain_release_ms);

    /* Verify voices parsed with instruments */
    if (music.voice_count != 4) {
        printf("FAIL DSL: expected 4 voices, got %d\n", music.voice_count);
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

static void test_seq_song_mapping(void)
{
    const char *path = "data/music/dark_moroder.abc";
    AbcMusic music;
    SeqSong song;

    if (abc_load(path, &music) != 0) {
        failf("seq mapping", "could not load dark_moroder.abc");
        return;
    }
    if (abc_build_seq_song(&music, &song) != 0) {
        failf("seq mapping", "abc_build_seq_song failed");
        return;
    }

    if (song.tempo_bpm != 124 || song.swing_pct != 58) {
        printf("FAIL seq mapping: got tempo=%d swing=%d\n", song.tempo_bpm, song.swing_pct);
        g_failures++;
    }
    if (song.fx_bus_count != 2) {
        printf("FAIL seq mapping: got fx_bus_count=%d expected 2\n", song.fx_bus_count);
        g_failures++;
    }
    if (song.arrangement_length != 8 || song.pattern_count != 8) {
        printf("FAIL seq mapping: got arrangement=%d patterns=%d expected 8/8\n",
               song.arrangement_length, song.pattern_count);
        g_failures++;
    }
    if (song.patterns[0].length != 16 || song.patterns[1].length != 16) {
        printf("FAIL seq mapping: unexpected pattern lengths %d/%d\n",
               song.patterns[0].length, song.patterns[1].length);
        g_failures++;
    }
    if (song.patterns[0].tracks[0].steps[0].note == SEQ_NOTE_REST ||
        song.patterns[0].tracks[1].steps[0].velocity == 0 ||
        song.patterns[0].tracks[1].steps[0].gate == 0 ||
        song.patterns[0].tracks[1].steps[0].fx_trigger == 0) {
        failf("seq mapping", "first pattern step was not fully populated");
    }
    if (song.patterns[0].tracks[3].steps[0].note != SEQ_NOTE_REST ||
        song.patterns[2].tracks[3].steps[8].note == SEQ_NOTE_REST) {
        failf("seq mapping", "lead rests and notes were not preserved");
    }
}

static void test_legacy_render_api(void)
{
    static const uint64_t expected_checksum = 0x0de5a3a1954d471bull;
    unsigned char *pcm;
    AudioRenderStats stats;
    int pcm_len = 0;

    pcm = audio_engine_render_abc_file("tests/fixtures/golden_small.abc", SAMPLE_RATE_ABC, &pcm_len, &stats);
    if (!pcm) {
        failf("legacy render api", "audio_engine_render_abc_file returned NULL");
        return;
    }
    if (pcm_len != 44100 || stats.sample_count != 44100ull || stats.checksum != expected_checksum) {
        printf("FAIL legacy render api: len=%d sample_count=%llu checksum=0x%016llx\n",
               pcm_len, stats.sample_count, (unsigned long long)stats.checksum);
        g_failures++;
    }
    if (stats.clipping_count > 1000) {
        printf("FAIL legacy render api: clipping=%llu exceeds max 1000\n", stats.clipping_count);
        g_failures++;
    }
    audio_engine_free_buffer(pcm);
}

static void test_invalid_dsl_handling(void)
{
    AbcMusic music;
    SeqSong song;
    unsigned char *pcm;
    int pcm_len = 0;

    if (abc_load("tests/fixtures/invalid_dsl.abc", &music) != 0) {
        failf("invalid dsl", "could not load invalid_dsl.abc");
        return;
    }
    if (abc_build_seq_song(&music, &song) == 0) {
        failf("invalid dsl", "abc_build_seq_song unexpectedly succeeded");
    }
    pcm = audio_engine_render_abc_file("tests/fixtures/invalid_dsl.abc", SAMPLE_RATE_ABC, &pcm_len, NULL);
    if (pcm != NULL || pcm_len != 0) {
        failf("invalid dsl", "invalid DSL rendered successfully");
        audio_engine_free_buffer(pcm);
    }
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
    if (music.instrument_count != 5) {
        printf("FAIL Advanced DSL: expected 5 instruments, got %d\n", music.instrument_count);
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
        if (music.fx_buses[0].delay_steps != 2 || music.fx_buses[0].mix_percent != 92) {
            printf("FAIL Multi-FX: bus 0 configuration incorrect\n");
            g_failures++;
        } else {
            printf("    Bus 0: delay=%d mix=%d (dry/punchy) ✓\n",
                   music.fx_buses[0].delay_steps, music.fx_buses[0].mix_percent);
        }
        
        /* Bus 1: wet/ambient */
        if (music.fx_buses[1].delay_steps != 8 || music.fx_buses[1].mix_percent != 62) {
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

    printf("\n");
    test_seq_song_mapping();

    printf("\n");
    test_legacy_render_api();

    printf("\n");
    test_invalid_dsl_handling();

    printf("\n");
    test_showcase_demo_regression();

    if (g_failures > 0) ok = 0;
    printf("\n%s\n", ok ? "All tests passed." : "Some tests FAILED.");
    return ok ? 0 : 1;
}
