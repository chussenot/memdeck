#ifndef MEMDECK_AUDIO_SEQ_H
#define MEMDECK_AUDIO_SEQ_H

#include <stdint.h>

#define SEQ_MAX_STEPS        64
#define SEQ_MAX_TRACKS       4
#define SEQ_MAX_PATTERNS     8
#define SEQ_MAX_ARRANGEMENT  16
#define SEQ_MAX_INSTRUMENTS  8
#define SEQ_MAX_FX_BUSES     4
#define SEQ_MAX_TIMELINE_STEPS (SEQ_MAX_STEPS * SEQ_MAX_ARRANGEMENT)

#define SEQ_NOTE_REST (-1)

typedef struct {
    int drive;
    int mix_percent;
} SeqFxBus;

typedef struct {
    int oscillator;
    int envelope_gate;
    int modulation;
    int fx_send;
    int duty_cycle;
    int accent_gain;
    int amplitude;
} SeqInstrument;

typedef struct {
    int16_t note;
    uint8_t velocity;
    uint8_t gate;
    uint8_t accent;
    uint8_t fx_trigger;
} SeqStep;

typedef struct {
    int instrument;
    int8_t automation[SEQ_MAX_STEPS];
    SeqStep steps[SEQ_MAX_STEPS];
} SeqTrack;

typedef struct {
    int length;
    int track_count;
    SeqTrack tracks[SEQ_MAX_TRACKS];
} SeqPattern;

typedef struct {
    char title[128];
    int tempo_bpm;
    int swing_pct;
    int steps_per_beat;
    int instrument_count;
    SeqInstrument instruments[SEQ_MAX_INSTRUMENTS];
    int pattern_count;
    SeqPattern patterns[SEQ_MAX_PATTERNS];
    int arrangement_length;
    uint8_t arrangement[SEQ_MAX_ARRANGEMENT];
    int fx_bus_count;
    SeqFxBus fx_buses[SEQ_MAX_FX_BUSES];
} SeqSong;

typedef struct {
    int pattern_index;
    int pattern_step;
    int samples;
} SeqTimelineStep;

typedef struct {
    int total_steps;
    int total_samples;
    int max_track_count;
    SeqTimelineStep steps[SEQ_MAX_TIMELINE_STEPS];
} SeqTimeline;

int seq_song_total_steps(const SeqSong *song);
int seq_compile_timeline(const SeqSong *song, int sample_rate, SeqTimeline *timeline);

#endif
