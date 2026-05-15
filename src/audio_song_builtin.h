#ifndef MEMDECK_AUDIO_SONG_BUILTIN_H
#define MEMDECK_AUDIO_SONG_BUILTIN_H

#include "audio_seq.h"

typedef enum {
    MEMDECK_PRESET_BASS_PULSE = 0,
    MEMDECK_PRESET_DARK_ARP,
    MEMDECK_PRESET_SOFT_PAD,
    MEMDECK_PRESET_BRASS_STAB,
    MEMDECK_PRESET_LEAD,
    MEMDECK_PRESET_KICK,
    MEMDECK_PRESET_HAT,
    MEMDECK_PRESET_NOISE_SNARE,
    MEMDECK_PRESET_COUNT
} MemdeckInstrumentPreset;

const SeqSong *audio_builtin_menu_song(void);
const SeqInstrument *audio_builtin_instrument_presets(int *count);
const char *audio_builtin_instrument_preset_name(int index);

#endif
