#include <stdlib.h>
#include <string.h>

#include "../src/audio_dsp.h"
#include "../src/memdeck.h"

int main(void)
{
    AbcMusic music;
    SeqSong song;
    AudioRenderStats stats;
    int pcm_len = 0;
    unsigned char *pcm;

    memset(&music, 0, sizeof(music));
    music.bpm = 120;
    music.step_ms = 125;
    music.voice_count = 2;

    snprintf(music.voices[0].name, sizeof(music.voices[0].name), "%s", "bass");
    music.voices[0].amplitude = 40;
    music.voices[0].waveform = DSP_WAVE_SQUARE;
    music.voices[0].duty_cycle = 25;
    music.voices[0].freqs[0] = 261.625565;
    music.voices[0].freqs[1] = 261.625565;
    music.voices[0].note_count = 2;

    snprintf(music.voices[1].name, sizeof(music.voices[1].name), "%s", "lead");
    music.voices[1].amplitude = 20;
    music.voices[1].staccato = 1;
    music.voices[1].waveform = DSP_WAVE_PULSE;
    music.voices[1].duty_cycle = 25;
    music.voices[1].freqs[0] = 329.627557;
    music.voices[1].freqs[1] = 329.627557;
    music.voices[1].note_count = 2;

    if (abc_build_seq_song(&music, &song) != 0)
        return 1;

    pcm = audio_engine_render_song(&song, SAMPLE_RATE_ABC, &pcm_len, &stats);
    if (!pcm || pcm_len <= 0 || stats.sample_count != (unsigned long long)pcm_len)
        return 1;

    audio_engine_free_buffer(pcm);
    return 0;
}
