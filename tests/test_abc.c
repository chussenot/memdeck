/*
 * Quick verification that the ABC parser produces correct frequencies
 * for the menu music voices.
 *
 * Build: cc -Wall -O2 -std=c99 -D_DEFAULT_SOURCE -o test_abc tests/test_abc.c src/abc.c src/card.c -lm -lncursesw
 * Run:   ./test_abc
 */
#include <stdio.h>
#include <math.h>
#include "../src/memdeck.h"

/* Expected frequencies from the hardcoded arrays in sound.c */
#define D2   73.42
#define Bb1  58.27
#define A1   55.00

#define A3  220.00
#define Bb3 233.08
#define C4  261.63
#define D4  293.66
#define E4  329.63
#define F4  349.23
#define A4  440.00
#define Bb4 466.16

#define C5  523.25
#define D5  587.33
#define E5  659.25
#define F5  698.46

static int check_freq(double got, double expect, const char *label, int step)
{
    if (expect == 0.0 && got == 0.0) return 1;
    if (expect == 0.0 || got == 0.0) {
        printf("FAIL %s step %d: got %.2f expected %.2f\n", label, step, got, expect);
        return 0;
    }
    double ratio = got / expect;
    if (ratio < 0.98 || ratio > 1.02) {
        printf("FAIL %s step %d: got %.2f expected %.2f (ratio %.4f)\n",
               label, step, got, expect, ratio);
        return 0;
    }
    return 1;
}

int main(void)
{
    AbcMusic music;
    const char *paths[3] = {
        "data/music/menu_bass.abc",
        "data/music/menu_arp.abc",
        "data/music/menu_lead.abc"
    };

    if (abc_load_voices(paths, 3, &music) != 0) {
        printf("FAIL: could not load ABC files\n");
        return 1;
    }

    printf("Loaded %d voices, BPM=%d, step_ms=%d\n",
           music.voice_count, music.bpm, music.step_ms);

    if (music.voice_count < 3) {
        printf("FAIL: expected 3 voices, got %d\n", music.voice_count);
        return 1;
    }

    /* Check bass voice - first 16 steps should all be D2 */
    int ok = 1;
    AbcVoice *bass = &music.voices[0];
    printf("Bass: %d notes, amp=%d\n", bass->note_count, bass->amplitude);
    for (int i = 0; i < 16 && i < bass->note_count; i++)
        ok &= check_freq(bass->freqs[i], D2, "bass", i);

    /* Steps 16-23 should be Bb1 */
    for (int i = 16; i < 24 && i < bass->note_count; i++)
        ok &= check_freq(bass->freqs[i], Bb1, "bass", i);

    /* Steps 24-55 should be A1 */
    for (int i = 24; i < 56 && i < bass->note_count; i++)
        ok &= check_freq(bass->freqs[i], A1, "bass", i);

    /* Check arp voice - first 4 steps: D4, F4, A4, D5 */
    AbcVoice *arp = &music.voices[1];
    printf("Arp: %d notes, amp=%d, staccato=%d\n", arp->note_count, arp->amplitude, arp->staccato);
    double arp_expect[4] = { D4, F4, A4, D5 };
    for (int i = 0; i < 4 && i < arp->note_count; i++)
        ok &= check_freq(arp->freqs[i], arp_expect[i], "arp", i);

    /* Check lead voice - first 8 should be rest, then D5, D5 */
    AbcVoice *lead = &music.voices[2];
    printf("Lead: %d notes, amp=%d\n", lead->note_count, lead->amplitude);
    for (int i = 0; i < 8 && i < lead->note_count; i++)
        ok &= check_freq(lead->freqs[i], 0.0, "lead", i);
    if (lead->note_count > 9) {
        ok &= check_freq(lead->freqs[8], D5, "lead", 8);
        ok &= check_freq(lead->freqs[9], D5, "lead", 9);
    }

    /* Check step_ms: at 120 BPM with L:1/16, step = 125ms */
    if (music.step_ms != 125) {
        printf("FAIL: step_ms=%d expected 125\n", music.step_ms);
        ok = 0;
    }

    /* Generate PCM and check it's non-empty */
    int pcm_len = 0;
    unsigned char *pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len == 0) {
        printf("FAIL: PCM generation failed\n");
        ok = 0;
    } else {
        printf("PCM: %d samples (%.2f seconds)\n", pcm_len, pcm_len / 22050.0);
        free(pcm);
    }

    if (ok)
        printf("\nAll ABC parser tests passed.\n");
    else
        printf("\nSome tests FAILED.\n");

    return ok ? 0 : 1;
}
