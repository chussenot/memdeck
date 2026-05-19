/*
 * memdeck-jam — infinite continuation player.
 *
 * Loads an ABC file once, then in a loop:
 *   1. Slice the next ~30s window of the song's arrangement
 *      (scrolls forward each iteration, wrapping at the end).
 *   2. Apply variation (velocity jitter, arrangement shuffle,
 *      drum fill, voice mute — see audio_jam.c).
 *   3. Render to PCM and write to stdout.
 *
 * PCM format: u8, 22050 Hz, mono. Pipe to your audio player:
 *
 *   bin/memdeck-jam data/music/three_chord_howl.abc | \
 *     aplay -q -f U8 -r 22050 -c 1
 *
 * Stops on SIGINT (Ctrl-C in the terminal) or SIGPIPE (when the
 * downstream pipe closes).
 */

#define _GNU_SOURCE
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "memdeck.h"
#include "audio_engine.h"
#include "audio_seq.h"
#include "audio_jam.h"

static volatile sig_atomic_t g_running = 1;
static void on_signal(int sig) { (void)sig; g_running = 0; }

static void usage(const char *argv0)
{
    fprintf(stderr,
        "Usage: %s <abc-file> [--seed N] [--section-seconds S]\n"
        "\n"
        "Streams variations of an ABC song to stdout as u8 mono 22050 Hz PCM,\n"
        "advancing through the arrangement in S-second chunks (default 30).\n"
        "Pipe to your audio player; Ctrl-C to stop.\n"
        "\n"
        "  --seed N            deterministic PRNG seed (default: time-based)\n"
        "  --section-seconds S target chunk length in seconds (default: 30)\n",
        argv0);
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        usage(argv[0]);
        return 1;
    }

    const char *path = argv[1];
    uint64_t seed = (uint64_t)time(NULL);
    double section_seconds = 30.0;

    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--seed") == 0 && i + 1 < argc) {
            seed = strtoull(argv[++i], NULL, 0);
        } else if (strcmp(argv[i], "--section-seconds") == 0 && i + 1 < argc) {
            section_seconds = strtod(argv[++i], NULL);
            if (section_seconds <= 0.0) section_seconds = 30.0;
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            usage(argv[0]);
            return 0;
        } else {
            fprintf(stderr, "Unknown argument: %s\n", argv[i]);
            usage(argv[0]);
            return 1;
        }
    }

    AbcMusic music;
    if (abc_load(path, &music) != 0) {
        fprintf(stderr, "Failed to load: %s\n", path);
        return 1;
    }
    SeqSong base;
    if (abc_build_seq_song(&music, &base) != 0) {
        fprintf(stderr, "Failed to build song from: %s\n", path);
        return 1;
    }
    if (base.arrangement_length <= 0) {
        fprintf(stderr, "Song has no arrangement; nothing to jam.\n");
        return 1;
    }

    int slots_per_section = audio_jam_slots_for_section(&base, section_seconds);
    if (slots_per_section < 1) slots_per_section = 1;

    JamState jam;
    audio_jam_init(&jam, seed);

    signal(SIGINT, on_signal);
    signal(SIGTERM, on_signal);
    signal(SIGPIPE, on_signal);

    fprintf(stderr,
        "[jam] %s\n"
        "[jam] seed=%lu  section=%.1fs  arrangement_slots/section=%d\n"
        "[jam] streaming u8 mono %d Hz PCM to stdout; Ctrl-C to stop.\n",
        music.title[0] ? music.title : "(untitled)",
        (unsigned long)seed, section_seconds, slots_per_section,
        SAMPLE_RATE_ABC);

    while (g_running) {
        SeqSong working;
        audio_jam_slice_song(&working, &base, jam.arrangement_offset, slots_per_section);
        audio_jam_vary_song(&working, &jam);

        int pcm_len = 0;
        AudioRenderStats stats;
        unsigned char *pcm = audio_engine_render_song(&working, SAMPLE_RATE_ABC,
                                                      &pcm_len, &stats);
        if (!pcm || pcm_len <= 0) {
            if (pcm) audio_engine_free_buffer(pcm);
            fprintf(stderr, "[jam] render failed at section %d; stopping.\n", jam.iteration);
            break;
        }

        /* Stream to stdout, honouring signals mid-write. */
        size_t to_write = (size_t)pcm_len;
        size_t written = 0;
        while (written < to_write && g_running) {
            size_t n = fwrite(pcm + written, 1, to_write - written, stdout);
            if (n == 0) { g_running = 0; break; }
            written += n;
        }
        fflush(stdout);
        audio_engine_free_buffer(pcm);

        fprintf(stderr, "[jam] section %d: %d samples (%.1fs), offset=%d, clipping=%llu\n",
                jam.iteration, pcm_len, pcm_len / (double)SAMPLE_RATE_ABC,
                jam.arrangement_offset, (unsigned long long)stats.clipping_count);

        /* Advance the scroll head. Wraps automatically inside slice_song. */
        jam.arrangement_offset += slots_per_section;
    }

    fprintf(stderr, "[jam] stopped after %d sections.\n", jam.iteration);
    return 0;
}
