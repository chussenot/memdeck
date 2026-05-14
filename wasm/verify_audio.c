#include <stdlib.h>
#include <string.h>

#include "../src/memdeck.h"

int main(void)
{
    AbcMusic music;
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

    pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len <= 0)
        return 1;

    free(pcm);
    return 0;
}
