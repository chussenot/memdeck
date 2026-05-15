#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../src/memdeck.h"
#include "../src/audio_mix.h"
#include "../src/audio_fx.h"
#include "../src/audio_song_builtin.h"

static int g_failures = 0;

static void check_int(const char *label, int got, int expected)
{
    if (got != expected) {
        printf("FAIL %s: got %d, expected %d\n", label, got, expected);
        g_failures++;
    }
}

static void check_true(const char *label, int ok)
{
    if (!ok) {
        printf("FAIL %s\n", label);
        g_failures++;
    }
}

static uint64_t fnv1a64(const unsigned char *data, int len)
{
    uint64_t hash = 1469598103934665603ull;
    for (int i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= 1099511628211ull;
    }
    return hash;
}

static unsigned long long pcm_sum_absolute_offset(const unsigned char *pcm, int len)
{
    unsigned long long e = 0;
    for (int i = 0; i < len; i++) {
        int d = (int)pcm[i] - 128;
        e += (unsigned long long)(d < 0 ? -d : d);
    }
    return e;
}

static void test_preset_initialization(void)
{
    int count = 0;
    const SeqInstrument *presets = audio_builtin_instrument_presets(&count);
    check_int("preset_count", count, MEMDECK_PRESET_COUNT);
    check_true("preset_ptr", presets != NULL);
    check_true("preset_name_bass",
               strcmp(audio_builtin_instrument_preset_name(MEMDECK_PRESET_BASS_PULSE), "memdeck_bass_pulse") == 0);
    check_true("preset_name_snare",
               strcmp(audio_builtin_instrument_preset_name(MEMDECK_PRESET_NOISE_SNARE), "memdeck_noise_snare") == 0);
    check_true("preset_has_adsr", presets[MEMDECK_PRESET_BASS_PULSE].envelope.attack_ms >= 0);
    check_true("preset_gate_valid",
               presets[MEMDECK_PRESET_DARK_ARP].envelope.gate_percent > 0 &&
               presets[MEMDECK_PRESET_DARK_ARP].envelope.gate_percent <= 100);
}

static void test_builtin_timeline(void)
{
    const SeqSong *song = audio_builtin_menu_song();
    SeqTimeline timeline;

    check_int("builtin_total_steps", seq_song_total_steps(song), 64);
    check_int("builtin_compile", seq_compile_timeline(song, 22050, &timeline), 0);
    check_int("builtin_timeline_steps", timeline.total_steps, 64);
    check_int("builtin_timeline_tracks", timeline.max_track_count, 4);
    check_int("builtin_timeline_samples", timeline.total_samples, 176400);
    check_int("builtin_pattern_0_start", timeline.steps[0].pattern_index, 0);
    check_int("builtin_pattern_0_end", timeline.steps[15].pattern_index, 0);
    check_int("builtin_pattern_1_start", timeline.steps[16].pattern_index, 1);
    check_int("builtin_pattern_2_start", timeline.steps[32].pattern_index, 2);
    check_int("builtin_pattern_3_start", timeline.steps[48].pattern_index, 3);
    check_int("builtin_pattern_3_end_step", timeline.steps[63].pattern_step, 15);
    check_int("builtin_track4_instrument",
              song->patterns[0].tracks[3].instrument, MEMDECK_PRESET_HAT);
    check_int("builtin_track4_step0_note",
              song->patterns[0].tracks[3].steps[0].note, 72);
    check_true("builtin_track4_has_activity",
               song->patterns[0].tracks[3].steps[0].velocity > 0);
}

static void test_note_events(void)
{
    const SeqSong *song = audio_builtin_menu_song();
    SeqTimeline timeline;
    SeqNoteEvent events[SEQ_MAX_STEP_EVENTS];
    int n;

    check_int("events_compile", seq_compile_timeline(song, 22050, &timeline), 0);
    n = seq_collect_step_events(song, &timeline, 0, events);
    check_true("events_step0_count", n >= 2);
    check_int("events_step0_first_track", events[0].track_index, 0);
    check_true("events_step0_gate", events[0].gate_percent > 0);
}

static void test_swing_timing(void)
{
    SeqSong song;
    SeqTimeline timeline;

    memset(&song, 0, sizeof(song));
    song.tempo_bpm = 120;
    song.swing_pct = 60;
    song.steps_per_beat = 4;
    song.instrument_count = 1;
    song.instruments[0].waveform = DSP_WAVE_SQUARE;
    song.instruments[0].pulse_width = 50;
    song.instruments[0].amplitude = 32;
    song.instruments[0].envelope.attack_ms = 0;
    song.instruments[0].envelope.decay_ms = 0;
    song.instruments[0].envelope.sustain_level = 100;
    song.instruments[0].envelope.release_ms = 0;
    song.instruments[0].envelope.gate_percent = 75;
    song.pattern_count = 1;
    song.patterns[0].length = 4;
    song.patterns[0].track_count = 1;
    song.patterns[0].tracks[0].instrument = 0;
    for (int i = 0; i < 4; i++) {
        song.patterns[0].tracks[0].steps[i].note = 60;
        song.patterns[0].tracks[0].steps[i].velocity = 96;
        song.patterns[0].tracks[0].steps[i].gate = 75;
    }
    song.arrangement_length = 1;
    song.arrangement[0] = 0;

    check_int("swing_compile", seq_compile_timeline(&song, 22050, &timeline), 0);
    check_true("swing_step_0_longer", timeline.steps[0].samples > timeline.steps[1].samples);
    check_true("swing_step_2_longer", timeline.steps[2].samples > timeline.steps[3].samples);
    check_true("swing_delta_large",
               (timeline.steps[0].samples - timeline.steps[1].samples) >= 1000);
    check_int("swing_total_samples", timeline.total_samples, 11025);
}

static void test_builtin_render(void)
{
    static const unsigned char expected_prefix[16] = {
        155, 156, 106, 156, 106, 156, 107, 157,
        109, 107, 158, 111, 109, 148, 149, 100
    };
    static const uint64_t expected_checksum = 0x1cc9f7453a231fafull;
    AudioRenderStats stats;
    unsigned char *pcm = NULL;
    int pcm_len = 0;

    pcm = audio_engine_render_builtin_menu(22050, &pcm_len, &stats);
    check_true("builtin_pcm_alloc", pcm != NULL);
    if (!pcm) return;

    check_int("builtin_pcm_len", pcm_len, 176400);
    check_true("builtin_stats_sample_count", stats.sample_count == (unsigned long long)pcm_len);
    check_true("builtin_stats_duration", stats.duration_ms > 7999.0 && stats.duration_ms < 8001.0);
    check_true("builtin_stats_clipping", stats.clipping_count < 8000);
    if (stats.checksum != expected_checksum) {
        printf("FAIL builtin_checksum: got 0x%016llx, expected 0x%016llx\n",
               (unsigned long long)stats.checksum, (unsigned long long)expected_checksum);
        g_failures++;
    }
    check_true("builtin_stats_checksum_matches_pcm", stats.checksum == fnv1a64(pcm, pcm_len));
    if (memcmp(pcm, expected_prefix, sizeof(expected_prefix)) != 0) {
        printf("FAIL builtin_prefix\n");
        g_failures++;
    }

    audio_engine_free_buffer(pcm);
}

static void test_fx_delay_circular(void)
{
    AudioDelay delay;
    int in[8] = { 1000, 0, 0, 0, 0, 0, 0, 0 };
    int out[8];

    check_int("delay_init", audio_fx_delay_init(&delay, 1000, 3, 50, 100), 0);
    for (int i = 0; i < 8; i++)
        out[i] = audio_fx_delay_process(&delay, in[i]);
    check_int("delay_o0", out[0], 0);
    check_int("delay_o3", out[3], 1000);
    check_int("delay_o6", out[6], 500);
    audio_fx_delay_free(&delay);
}

static void test_fx_drive_clamp(void)
{
    int y0 = audio_fx_apply_drive(200, 0);
    int y1 = audio_fx_apply_drive(200, 40);
    int y2 = audio_fx_apply_drive(20000, 100);
    check_int("drive_bypass", y0, 200);
    check_true("drive_gain", y1 > y0);
    check_true("drive_clamp", y2 <= 2047);
}

static void test_fx_lowpass_stability(void)
{
    AudioLowpass lp;
    int y = 0;
    int prev = -32768;
    audio_fx_lowpass_init(&lp, 60);
    for (int i = 0; i < 400; i++) {
        y = audio_fx_lowpass_process(&lp, 1000);
        check_true("lowpass_monotonic", y >= prev);
        check_true("lowpass_bound", y <= 1000);
        prev = y;
    }
    check_true("lowpass_converges", y >= 900);
}

static void test_fx_sidechain_decay(void)
{
    AudioSidechain sc;
    int env = 32767;
    int y0;
    int y1 = 0;
    audio_fx_sidechain_init(&sc, 1000, 40, 100);
    y0 = audio_fx_sidechain_process(&sc, 1000, 1, &env);
    for (int i = 0; i < 120; i++)
        y1 = audio_fx_sidechain_process(&sc, 1000, 0, &env);
    check_true("sidechain_duck", y0 < 800);
    check_true("sidechain_recover", y1 > y0);
    check_true("sidechain_release", y1 >= 980);
}

static void test_builtin_render_fx_disabled(void)
{
    static const unsigned char expected_prefix[16] = {
        168, 168, 91, 170, 93, 172, 95, 173,
        98, 98, 176, 101, 101, 161, 160, 85
    };
    static const uint64_t expected_checksum = 0xd138a815d4455aefull;
    SeqSong song = *audio_builtin_menu_song();
    AudioRenderStats stats;
    unsigned char *pcm = NULL;
    int pcm_len = 0;

    memset(song.fx_buses, 0, sizeof(SeqFxBus) * (size_t)song.fx_bus_count);
    pcm = audio_engine_render_song(&song, 22050, &pcm_len, &stats);
    check_true("builtin_fx_off_pcm_alloc", pcm != NULL);
    if (!pcm) return;
    check_int("builtin_fx_off_pcm_len", pcm_len, 176400);
    check_true("builtin_fx_off_stats_sample_count", stats.sample_count == (unsigned long long)pcm_len);
    check_true("builtin_fx_off_stats_clipping", stats.clipping_count < 8000);
    if (stats.checksum != expected_checksum) {
        printf("FAIL builtin_fx_off_checksum: got 0x%016llx, expected 0x%016llx\n",
               (unsigned long long)stats.checksum, (unsigned long long)expected_checksum);
        g_failures++;
    }
    if (memcmp(pcm, expected_prefix, sizeof(expected_prefix)) != 0) {
        printf("FAIL builtin_fx_off_prefix\n");
        g_failures++;
    }
    audio_engine_free_buffer(pcm);
}

static void test_accent_triggers_fx_sidechain(void)
{
    SeqSong song;
    unsigned char *plain_pcm;
    unsigned char *accent_pcm;
    int plain_len = 0;
    int accent_len = 0;
    unsigned long long plain_energy;
    unsigned long long accent_energy;

    memset(&song, 0, sizeof(song));
    snprintf(song.title, sizeof(song.title), "accent_trigger_test");
    song.tempo_bpm = 120;
    song.swing_pct = 50;
    song.steps_per_beat = 4;
    song.instrument_count = 1;
    song.instruments[0].waveform = DSP_WAVE_SQUARE;
    song.instruments[0].amplitude = 96;
    song.instruments[0].pulse_width = 50;
    song.instruments[0].envelope.attack_ms = 0;
    song.instruments[0].envelope.decay_ms = 0;
    song.instruments[0].envelope.sustain_level = 100;
    song.instruments[0].envelope.release_ms = 0;
    song.instruments[0].envelope.gate_percent = 100;
    song.instruments[0].fx_send = 0;
    song.pattern_count = 1;
    song.patterns[0].length = 1;
    song.patterns[0].track_count = 1;
    song.patterns[0].tracks[0].instrument = 0;
    song.patterns[0].tracks[0].steps[0].note = 60;
    song.patterns[0].tracks[0].steps[0].velocity = 120;
    song.patterns[0].tracks[0].steps[0].gate = 100;
    song.patterns[0].tracks[0].steps[0].fx_trigger = 0;
    song.arrangement_length = 1;
    song.arrangement[0] = 0;
    song.fx_bus_count = 1;
    song.fx_buses[0].enabled = 1;
    song.fx_buses[0].sidechain_amount = 45;
    song.fx_buses[0].sidechain_release_ms = 180;
    song.fx_buses[0].mix_percent = 0;

    song.patterns[0].tracks[0].steps[0].accent = 0;
    plain_pcm = audio_mix_render_song(&song, 22050, &plain_len);
    song.patterns[0].tracks[0].steps[0].accent = 1;
    accent_pcm = audio_mix_render_song(&song, 22050, &accent_len);

    check_true("accent_plain_pcm", plain_pcm != NULL);
    check_true("accent_fx_pcm", accent_pcm != NULL);
    if (!plain_pcm || !accent_pcm) {
        free(plain_pcm);
        free(accent_pcm);
        return;
    }
    check_int("accent_len_match", accent_len, plain_len);
    plain_energy = pcm_sum_absolute_offset(plain_pcm, plain_len);
    accent_energy = pcm_sum_absolute_offset(accent_pcm, accent_len);
    check_true("accent_triggers_pump", accent_energy < plain_energy);
    free(plain_pcm);
    free(accent_pcm);
}

int main(void)
{
    printf("=== Audio sequencer regression tests ===\n");

    test_preset_initialization();
    test_builtin_timeline();
    test_note_events();
    test_swing_timing();
    test_fx_delay_circular();
    test_fx_drive_clamp();
    test_fx_lowpass_stability();
    test_fx_sidechain_decay();
    test_accent_triggers_fx_sidechain();
    test_builtin_render();
    test_builtin_render_fx_disabled();

    printf("Audio sequencer regression: %d failure(s)\n", g_failures);
    printf("%s\n", g_failures == 0 ? "All tests passed." : "Some tests FAILED.");
    return g_failures > 0 ? 1 : 0;
}
