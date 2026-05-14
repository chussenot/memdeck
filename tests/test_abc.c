/*
 * Verify ABC parser for both menu tracks.
 * Build (via Makefile target "test-abc"):
 *   $(CC) $(CFLAGS) -o bin/test-abc tests/test_abc.c src/abc.c src/card.c src/audio_dsp.c $(LDFLAGS)
 */
#include <stdio.h>
#include <math.h>
#include "../src/memdeck.h"

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

    printf("\n%s\n", ok ? "All tests passed." : "Some tests FAILED.");
    return ok ? 0 : 1;
}
