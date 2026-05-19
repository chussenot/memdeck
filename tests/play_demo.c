#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../src/audio_engine.h"
#include "../src/memdeck.h"

typedef struct {
    const char *name;
    const char *path;
} DemoEntry;

static const DemoEntry kDemos[] = {
    { "dark_moroder", "data/music/dark_moroder.abc" },
    { "perturbator_loop", "data/music/perturbator_loop.abc" },
    { "carpenter_drive", "data/music/carpenter_drive.abc" },
    { "advanced_dsl_demo", "data/music/advanced_dsl_demo.abc" },
    { "multi_fx_demo", "data/music/multi_fx_demo.abc" },
    { "neon_nightdrive", "data/music/neon_nightdrive.abc" },
    { "metro_chase", "data/music/metro_chase.abc" },
    { "black_sunrise", "data/music/black_sunrise.abc" },
    { "machine_romance", "data/music/machine_romance.abc" },
    { "hypersleep_dream", "data/music/hypersleep_dream.abc" },
    { "aurora_halo", "data/music/aurora_halo.abc" },
    { "glass_anthem", "data/music/glass_anthem.abc" },
    { "pixie_dust", "data/music/pixie_dust.abc" },
    { "surrender_loop", "data/music/surrender_loop.abc" },
    { "moog_lattice", "data/music/moog_lattice.abc" }
};

static void write_u16_le(FILE *f, uint16_t v)
{
    fputc((int)(v & 0xff), f);
    fputc((int)((v >> 8) & 0xff), f);
}

static void write_u32_le(FILE *f, uint32_t v)
{
    fputc((int)(v & 0xff), f);
    fputc((int)((v >> 8) & 0xff), f);
    fputc((int)((v >> 16) & 0xff), f);
    fputc((int)((v >> 24) & 0xff), f);
}

static int write_wav_u8_mono(const char *path, const unsigned char *pcm, int len, int sample_rate)
{
    FILE *f;
    uint32_t data_size;
    uint32_t riff_size;

    if (!path || !pcm || len <= 0 || sample_rate <= 0) return 1;
    f = fopen(path, "wb");
    if (!f) return 1;

    data_size = (uint32_t)len;
    riff_size = 36u + data_size;

    fwrite("RIFF", 1, 4, f);
    write_u32_le(f, riff_size);
    fwrite("WAVE", 1, 4, f);
    fwrite("fmt ", 1, 4, f);
    write_u32_le(f, 16);                 /* fmt chunk size */
    write_u16_le(f, 1);                  /* PCM */
    write_u16_le(f, 1);                  /* mono */
    write_u32_le(f, (uint32_t)sample_rate);
    write_u32_le(f, (uint32_t)sample_rate); /* byte rate (8-bit mono) */
    write_u16_le(f, 1);                  /* block align */
    write_u16_le(f, 8);                  /* bits/sample */
    fwrite("data", 1, 4, f);
    write_u32_le(f, data_size);
    fwrite(pcm, 1, (size_t)len, f);

    fclose(f);
    return 0;
}

static const char *resolve_demo_path(const char *demo)
{
    size_t i;
    if (!demo || !*demo) return NULL;
    for (i = 0; i < sizeof(kDemos) / sizeof(kDemos[0]); i++) {
        if (strcmp(kDemos[i].name, demo) == 0) return kDemos[i].path;
    }
    if (strstr(demo, ".abc") != NULL || strchr(demo, '/') != NULL) return demo;
    return NULL;
}

static void usage(void)
{
    size_t i;
    printf("Usage: play-demo <name|path.abc> [--wav output.wav]\n");
    printf("Available demos:\n");
    for (i = 0; i < sizeof(kDemos) / sizeof(kDemos[0]); i++)
        printf("  - %s\n", kDemos[i].name);
}

int main(int argc, char **argv)
{
    const char *path;
    const char *wav_out = NULL;
    AbcMusic music;
    AudioRenderStats stats;
    unsigned char *pcm;
    int pcm_len = 0;
    int i;

    if (argc < 2) {
        usage();
        return 1;
    }

    for (i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--wav") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Missing value for --wav\n");
                return 1;
            }
            wav_out = argv[i + 1];
            i++;
        } else {
            fprintf(stderr, "Unknown argument: %s\n", argv[i]);
            return 1;
        }
    }

    path = resolve_demo_path(argv[1]);
    if (!path) {
        fprintf(stderr, "Unknown demo: %s\n", argv[1]);
        usage();
        return 1;
    }
    if (abc_load(path, &music) != 0) {
        fprintf(stderr, "Could not parse demo: %s\n", path);
        return 1;
    }
    pcm = audio_engine_render_abc_file(path, SAMPLE_RATE_ABC, &pcm_len, &stats);
    if (!pcm || pcm_len <= 0) {
        fprintf(stderr, "Could not render demo: %s\n", path);
        return 1;
    }

    printf("%s | duration=%.3fs | pcm_len=%d | checksum=0x%016llx | clipping=%llu\n",
           music.title[0] ? music.title : path,
           stats.duration_ms / 1000.0,
           pcm_len,
           (unsigned long long)stats.checksum,
           stats.clipping_count);

    if (wav_out) {
        if (write_wav_u8_mono(wav_out, pcm, pcm_len, SAMPLE_RATE_ABC) != 0) {
            fprintf(stderr, "Failed to write WAV: %s\n", wav_out);
            audio_engine_free_buffer(pcm);
            return 1;
        }
        printf("WAV export: %s\n", wav_out);
    }

    audio_engine_free_buffer(pcm);
    return 0;
}
