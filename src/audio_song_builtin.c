#include "audio_song_builtin.h"
#include "audio_dsp.h"

#define N_C4   60
#define N_E4   64
#define N_F4   65
#define N_G4   67
#define N_A4   69
#define N_BB4  70
#define N_C5   72
#define N_D5   74
#define N_E5   76
#define N_F5   77

#define N_A1   33
#define N_BB1  34
#define N_D2   38

#define N_A3   57
#define N_BB3  58
#define N_C4L  60
#define N_D4   62
#define N_F4L  65
#define N_A4L  69
#define N_BB4L 70

#define N_C2   36

#define REST_STEP        { SEQ_NOTE_REST, 0,   0, 0, 0 }
#define BASS_STEP(n)     { n,             112, 92, 0, 0 }
#define ARP_STEP(n)      { n,              92, 68, 1, 1 }
#define LEAD_STEP(n)     { n,             108, 88, 0, 0 }
#define KICK_STEP        { N_C2,          120, 55, 0, 1 }
#define HAT_STEP         { N_C5,           76, 24, 0, 1 }

static const char *g_preset_names[MEMDECK_PRESET_COUNT] = {
    "memdeck_bass_pulse",
    "memdeck_dark_arp",
    "memdeck_soft_pad",
    "memdeck_brass_stab",
    "memdeck_lead",
    "memdeck_kick",
    "memdeck_hat",
    "memdeck_noise_snare"
};

static const SeqInstrument g_presets[MEMDECK_PRESET_COUNT] = {
    [MEMDECK_PRESET_BASS_PULSE] = {
        .waveform = DSP_WAVE_PULSE,
        .amplitude = 90,
        .pulse_width = 30,
        .envelope = { 6, 72, 60, 48, 88 },
        .detune_cents = 3,
        .vibrato_depth_cents = 0,
        .vibrato_rate = 0,
        .pwm_depth = 3,
        .pwm_rate = 1800,
        .glide_ms = 22,
        .noise_mode = 0,
        .stacked_voices = 2,
        .fx_send = -1,
        .accent_gain = 8
    },
    [MEMDECK_PRESET_DARK_ARP] = {
        .waveform = DSP_WAVE_PULSE,
        .amplitude = 70,
        .pulse_width = 24,
        .envelope = { 2, 36, 42, 40, 66 },
        .detune_cents = 6,
        .vibrato_depth_cents = 4,
        .vibrato_rate = 5200,
        .pwm_depth = 8,
        .pwm_rate = 6200,
        .glide_ms = 10,
        .noise_mode = 0,
        .stacked_voices = 2,
        .fx_send = 0,
        .accent_gain = 16
    },
    [MEMDECK_PRESET_SOFT_PAD] = {
        .waveform = DSP_WAVE_TRIANGLE,
        .amplitude = 52,
        .pulse_width = 50,
        .envelope = { 80, 120, 70, 140, 96 },
        .detune_cents = 3,
        .vibrato_depth_cents = 5,
        .vibrato_rate = 3400,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 40,
        .noise_mode = 0,
        .stacked_voices = 2,
        .fx_send = 0,
        .accent_gain = 4
    },
    [MEMDECK_PRESET_BRASS_STAB] = {
        .waveform = DSP_WAVE_SQUARE,
        .amplitude = 86,
        .pulse_width = 50,
        .envelope = { 2, 46, 35, 32, 70 },
        .detune_cents = 5,
        .vibrato_depth_cents = 2,
        .vibrato_rate = 4200,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 0,
        .noise_mode = 0,
        .stacked_voices = 2,
        .fx_send = -1,
        .accent_gain = 14
    },
    [MEMDECK_PRESET_LEAD] = {
        .waveform = DSP_WAVE_SQUARE,
        .amplitude = 84,
        .pulse_width = 50,
        .envelope = { 4, 56, 52, 48, 88 },
        .detune_cents = 2,
        .vibrato_depth_cents = 8,
        .vibrato_rate = 6200,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 18,
        .noise_mode = 0,
        .stacked_voices = 1,
        .fx_send = -1,
        .accent_gain = 10
    },
    [MEMDECK_PRESET_KICK] = {
        .waveform = DSP_WAVE_TRIANGLE,
        .amplitude = 108,
        .pulse_width = 50,
        .envelope = { 0, 34, 4, 24, 44 },
        .detune_cents = 0,
        .vibrato_depth_cents = 0,
        .vibrato_rate = 0,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 0,
        .noise_mode = 0,
        .stacked_voices = 1,
        .fx_send = 0,
        .accent_gain = 20
    },
    [MEMDECK_PRESET_HAT] = {
        .waveform = DSP_WAVE_NOISE,
        .amplitude = 68,
        .pulse_width = 50,
        .envelope = { 0, 12, 0, 10, 20 },
        .detune_cents = 0,
        .vibrato_depth_cents = 0,
        .vibrato_rate = 0,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 0,
        .noise_mode = 1,
        .stacked_voices = 1,
        .fx_send = 0,
        .accent_gain = 10
    },
    [MEMDECK_PRESET_NOISE_SNARE] = {
        .waveform = DSP_WAVE_NOISE,
        .amplitude = 82,
        .pulse_width = 50,
        .envelope = { 0, 28, 8, 20, 36 },
        .detune_cents = 0,
        .vibrato_depth_cents = 0,
        .vibrato_rate = 0,
        .pwm_depth = 0,
        .pwm_rate = 0,
        .glide_ms = 0,
        .noise_mode = 1,
        .stacked_voices = 1,
        .fx_send = 0,
        .accent_gain = 16
    },
};

static const SeqSong g_menu_song = {
    .title = "MemDeck Built-in Retro Sequencer",
    .tempo_bpm = 120,
    .swing_pct = 54,
    .steps_per_beat = 4,
    .instrument_count = MEMDECK_PRESET_COUNT,
    .instruments = {
        [MEMDECK_PRESET_BASS_PULSE] = {
            .waveform = DSP_WAVE_PULSE, .amplitude = 90, .pulse_width = 30, .envelope = { 6, 72, 60, 48, 88 },
            .detune_cents = 3, .vibrato_depth_cents = 0, .vibrato_rate = 0, .pwm_depth = 3, .pwm_rate = 1800,
            .glide_ms = 22, .noise_mode = 0, .stacked_voices = 2, .fx_send = -1, .accent_gain = 8
        },
        [MEMDECK_PRESET_DARK_ARP] = {
            .waveform = DSP_WAVE_PULSE, .amplitude = 70, .pulse_width = 24, .envelope = { 2, 36, 42, 40, 66 },
            .detune_cents = 6, .vibrato_depth_cents = 4, .vibrato_rate = 5200, .pwm_depth = 8, .pwm_rate = 6200,
            .glide_ms = 10, .noise_mode = 0, .stacked_voices = 2, .fx_send = 0, .accent_gain = 16
        },
        [MEMDECK_PRESET_SOFT_PAD] = {
            .waveform = DSP_WAVE_TRIANGLE, .amplitude = 52, .pulse_width = 50, .envelope = { 80, 120, 70, 140, 96 },
            .detune_cents = 3, .vibrato_depth_cents = 5, .vibrato_rate = 3400, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 40, .noise_mode = 0, .stacked_voices = 2, .fx_send = 0, .accent_gain = 4
        },
        [MEMDECK_PRESET_BRASS_STAB] = {
            .waveform = DSP_WAVE_SQUARE, .amplitude = 86, .pulse_width = 50, .envelope = { 2, 46, 35, 32, 70 },
            .detune_cents = 5, .vibrato_depth_cents = 2, .vibrato_rate = 4200, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 0, .noise_mode = 0, .stacked_voices = 2, .fx_send = -1, .accent_gain = 14
        },
        [MEMDECK_PRESET_LEAD] = {
            .waveform = DSP_WAVE_SQUARE, .amplitude = 84, .pulse_width = 50, .envelope = { 4, 56, 52, 48, 88 },
            .detune_cents = 2, .vibrato_depth_cents = 8, .vibrato_rate = 6200, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 18, .noise_mode = 0, .stacked_voices = 1, .fx_send = -1, .accent_gain = 10
        },
        [MEMDECK_PRESET_KICK] = {
            .waveform = DSP_WAVE_TRIANGLE, .amplitude = 108, .pulse_width = 50, .envelope = { 0, 34, 4, 24, 44 },
            .detune_cents = 0, .vibrato_depth_cents = 0, .vibrato_rate = 0, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 0, .noise_mode = 0, .stacked_voices = 1, .fx_send = 0, .accent_gain = 20
        },
        [MEMDECK_PRESET_HAT] = {
            .waveform = DSP_WAVE_NOISE, .amplitude = 68, .pulse_width = 50, .envelope = { 0, 12, 0, 10, 20 },
            .detune_cents = 0, .vibrato_depth_cents = 0, .vibrato_rate = 0, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 0, .noise_mode = 1, .stacked_voices = 1, .fx_send = 0, .accent_gain = 10
        },
        [MEMDECK_PRESET_NOISE_SNARE] = {
            .waveform = DSP_WAVE_NOISE, .amplitude = 82, .pulse_width = 50, .envelope = { 0, 28, 8, 20, 36 },
            .detune_cents = 0, .vibrato_depth_cents = 0, .vibrato_rate = 0, .pwm_depth = 0, .pwm_rate = 0,
            .glide_ms = 0, .noise_mode = 1, .stacked_voices = 1, .fx_send = 0, .accent_gain = 16
        },
    },
    .pattern_count = 4,
    .patterns = {
        {
            .length = 16,
            .track_count = 4,
            .tracks = {
                {
                    .instrument = MEMDECK_PRESET_BASS_PULSE,
                    .steps = {
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_DARK_ARP,
                    .automation = { 4, 6, 8, 10, 4, 6, 8, 10, 4, 6, 8, 10, 4, 6, 8, 10 },
                    .steps = {
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_LEAD,
                    .steps = {
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        LEAD_STEP(N_D5), LEAD_STEP(N_D5), REST_STEP,       REST_STEP,
                        LEAD_STEP(N_F5), LEAD_STEP(N_E5), LEAD_STEP(N_D5), LEAD_STEP(N_D5),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_HAT,
                    .steps = {
                        HAT_STEP, REST_STEP, HAT_STEP, REST_STEP, HAT_STEP, REST_STEP, HAT_STEP, REST_STEP,
                        HAT_STEP, REST_STEP, HAT_STEP, REST_STEP, HAT_STEP, REST_STEP, HAT_STEP, REST_STEP,
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 4,
            .tracks = {
                {
                    .instrument = MEMDECK_PRESET_BASS_PULSE,
                    .steps = {
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_DARK_ARP,
                    .automation = { 2, 4, 6, 8, 2, 4, 6, 8, 3, 5, 7, 9, 3, 5, 7, 9 },
                    .steps = {
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_BRASS_STAB,
                    .steps = {
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        LEAD_STEP(N_BB4), LEAD_STEP(N_BB4), REST_STEP,        REST_STEP,
                        LEAD_STEP(N_C5),  LEAD_STEP(N_C5),  REST_STEP,        REST_STEP,
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_KICK,
                    .steps = {
                        KICK_STEP, REST_STEP, REST_STEP, REST_STEP, KICK_STEP, REST_STEP, REST_STEP, REST_STEP,
                        KICK_STEP, REST_STEP, REST_STEP, REST_STEP, KICK_STEP, REST_STEP, REST_STEP, REST_STEP,
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 4,
            .tracks = {
                {
                    .instrument = MEMDECK_PRESET_BASS_PULSE,
                    .steps = {
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_DARK_ARP,
                    .automation = { 4, 5, 6, 7, 4, 5, 6, 7, 5, 7, 9, 11, 5, 7, 9, 11 },
                    .steps = {
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_LEAD,
                    .steps = {
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        LEAD_STEP(N_A4), REST_STEP,       LEAD_STEP(N_C5), REST_STEP,
                        LEAD_STEP(N_E5), LEAD_STEP(N_E5), LEAD_STEP(N_D5), LEAD_STEP(N_D5),
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_HAT,
                    .steps = {
                        HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP,
                        HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP, HAT_STEP,
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 4,
            .tracks = {
                {
                    .instrument = MEMDECK_PRESET_BASS_PULSE,
                    .steps = {
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_D2),  BASS_STEP(N_D2),  BASS_STEP(N_D2),  BASS_STEP(N_D2),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_DARK_ARP,
                    .automation = { 3, 5, 7, 9, 3, 5, 7, 9, 5, 7, 9, 11, 6, 8, 10, 12 },
                    .steps = {
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_LEAD,
                    .steps = {
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        LEAD_STEP(N_A4),  LEAD_STEP(N_A4),  LEAD_STEP(N_BB4), LEAD_STEP(N_A4),
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                    },
                },
                {
                    .instrument = MEMDECK_PRESET_KICK,
                    .steps = {
                        KICK_STEP, REST_STEP, KICK_STEP, REST_STEP, KICK_STEP, REST_STEP, KICK_STEP, REST_STEP,
                        KICK_STEP, REST_STEP, KICK_STEP, REST_STEP, KICK_STEP, REST_STEP, KICK_STEP, REST_STEP,
                    },
                },
            },
        },
    },
    .arrangement_length = 4,
    .arrangement = { 0, 1, 2, 3 },
    .fx_bus_count = 1,
    .fx_buses = {
        {
            .enabled = 1,
            .delay_steps = 3,
            .delay_feedback = 35,
            .delay_mix = 25,
            .drive_amount = 20,
            .lowpass_amount = 35,
            .sidechain_amount = 40,
            .sidechain_release_ms = 180,
            .mix_percent = 40,
        },
    },
};

const SeqSong *audio_builtin_menu_song(void)
{
    return &g_menu_song;
}

const SeqInstrument *audio_builtin_instrument_presets(int *count)
{
    if (count) *count = MEMDECK_PRESET_COUNT;
    return g_presets;
}

const char *audio_builtin_instrument_preset_name(int index)
{
    if (index < 0 || index >= MEMDECK_PRESET_COUNT) return "";
    return g_preset_names[index];
}
