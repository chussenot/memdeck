#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../src/audio_mix.h"
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
        168, 168, 91, 170, 93, 172, 95, 173,
        98, 98, 176, 101, 101, 161, 160, 85
    };
    static const uint64_t expected_checksum = 0xd138a815d4455aefull;
    const SeqSong *song = audio_builtin_menu_song();
    unsigned char *pcm = NULL;
    int pcm_len = 0;
    uint64_t checksum;

    pcm = audio_mix_render_song(song, 22050, &pcm_len);
    check_true("builtin_pcm_alloc", pcm != NULL);
    if (!pcm) return;

    check_int("builtin_pcm_len", pcm_len, 176400);
    checksum = fnv1a64(pcm, pcm_len);
    if (checksum != expected_checksum) {
        printf("FAIL builtin_checksum: got 0x%016llx, expected 0x%016llx\n",
               (unsigned long long)checksum, (unsigned long long)expected_checksum);
        g_failures++;
    }
    if (memcmp(pcm, expected_prefix, sizeof(expected_prefix)) != 0) {
        printf("FAIL builtin_prefix\n");
        g_failures++;
    }

    free(pcm);
}

int main(void)
{
    printf("=== Audio sequencer regression tests ===\n");

    test_preset_initialization();
    test_builtin_timeline();
    test_note_events();
    test_swing_timing();
    test_builtin_render();

    printf("Audio sequencer regression: %d failure(s)\n", g_failures);
    printf("%s\n", g_failures == 0 ? "All tests passed." : "Some tests FAILED.");
    return g_failures > 0 ? 1 : 0;
}
