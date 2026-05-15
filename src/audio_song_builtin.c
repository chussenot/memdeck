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

#define REST_STEP     { SEQ_NOTE_REST, 0,   0, 0, 0 }
#define BASS_STEP(n)  { n,             112, 92, 0, 0 }
#define ARP_STEP(n)   { n,              92, 68, 1, 1 }
#define LEAD_STEP(n)  { n,             108, 88, 0, 0 }

static const SeqSong g_menu_song = {
    .title = "MemDeck Built-in Retro Sequencer",
    .tempo_bpm = 120,
    .swing_pct = 54,
    .steps_per_beat = 4,
    .instrument_count = 3,
    .instruments = {
        { DSP_WAVE_SQUARE, 92, 0, -1, 50, 8, 48 },
        { DSP_WAVE_PULSE,  68, 6,  0, 25, 16, 32 },
        { DSP_WAVE_SQUARE, 88, 0, -1, 50, 10, 42 },
    },
    .pattern_count = 4,
    .patterns = {
        {
            .length = 16,
            .track_count = 3,
            .tracks = {
                {
                    .instrument = 0,
                    .steps = {
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                        BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2), BASS_STEP(N_D2),
                    },
                },
                {
                    .instrument = 1,
                    .automation = { 4, 6, 8, 10, 4, 6, 8, 10, 4, 6, 8, 10, 4, 6, 8, 10 },
                    .steps = {
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                    },
                },
                {
                    .instrument = 2,
                    .steps = {
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        LEAD_STEP(N_D5), LEAD_STEP(N_D5), REST_STEP,       REST_STEP,
                        LEAD_STEP(N_F5), LEAD_STEP(N_E5), LEAD_STEP(N_D5), LEAD_STEP(N_D5),
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 3,
            .tracks = {
                {
                    .instrument = 0,
                    .steps = {
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                    },
                },
                {
                    .instrument = 1,
                    .automation = { 2, 4, 6, 8, 2, 4, 6, 8, 3, 5, 7, 9, 3, 5, 7, 9 },
                    .steps = {
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                    },
                },
                {
                    .instrument = 2,
                    .steps = {
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        LEAD_STEP(N_BB4), LEAD_STEP(N_BB4), REST_STEP,        REST_STEP,
                        LEAD_STEP(N_C5),  LEAD_STEP(N_C5),  REST_STEP,        REST_STEP,
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 3,
            .tracks = {
                {
                    .instrument = 0,
                    .steps = {
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                        BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1), BASS_STEP(N_A1),
                    },
                },
                {
                    .instrument = 1,
                    .automation = { 4, 5, 6, 7, 4, 5, 6, 7, 5, 7, 9, 11, 5, 7, 9, 11 },
                    .steps = {
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                        ARP_STEP(N_A3), ARP_STEP(N_C4L), ARP_STEP(N_E4), ARP_STEP(N_A4L),
                    },
                },
                {
                    .instrument = 2,
                    .steps = {
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                        LEAD_STEP(N_A4), REST_STEP,       LEAD_STEP(N_C5), REST_STEP,
                        LEAD_STEP(N_E5), LEAD_STEP(N_E5), LEAD_STEP(N_D5), LEAD_STEP(N_D5),
                        REST_STEP,       REST_STEP,       REST_STEP,       REST_STEP,
                    },
                },
            },
        },
        {
            .length = 16,
            .track_count = 3,
            .tracks = {
                {
                    .instrument = 0,
                    .steps = {
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),  BASS_STEP(N_A1),
                        BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1), BASS_STEP(N_BB1),
                        BASS_STEP(N_D2),  BASS_STEP(N_D2),  BASS_STEP(N_D2),  BASS_STEP(N_D2),
                    },
                },
                {
                    .instrument = 1,
                    .automation = { 3, 5, 7, 9, 3, 5, 7, 9, 5, 7, 9, 11, 6, 8, 10, 12 },
                    .steps = {
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_A3),  ARP_STEP(N_C4L), ARP_STEP(N_E4),  ARP_STEP(N_A4L),
                        ARP_STEP(N_BB3), ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_BB4L),
                        ARP_STEP(N_D4),  ARP_STEP(N_F4L), ARP_STEP(N_A4L), ARP_STEP(N_D5),
                    },
                },
                {
                    .instrument = 2,
                    .steps = {
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                        LEAD_STEP(N_A4),  LEAD_STEP(N_A4),  LEAD_STEP(N_BB4), LEAD_STEP(N_A4),
                        REST_STEP,        REST_STEP,        REST_STEP,        REST_STEP,
                    },
                },
            },
        },
    },
    .arrangement_length = 4,
    .arrangement = { 0, 1, 2, 3 },
    .fx_bus_count = 1,
    .fx_buses = {
        { 24, 35 },
    },
};

const SeqSong *audio_builtin_menu_song(void)
{
    return &g_menu_song;
}
