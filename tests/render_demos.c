#include <stdio.h>

#include "../src/audio_engine.h"
#include "../src/memdeck.h"

static int render_file(const char *path)
{
    AbcMusic music;
    AudioRenderStats stats;
    unsigned char *pcm;
    int pcm_len = 0;

    if (abc_load(path, &music) != 0) {
        printf("%s | parse=FAIL\n", path);
        return 1;
    }
    pcm = audio_engine_render_abc_file(path, SAMPLE_RATE_ABC, &pcm_len, &stats);
    if (!pcm || pcm_len <= 0) {
        printf("%s | render=FAIL\n", path);
        return 1;
    }

    printf("%s | duration=%.3fs | pcm_len=%d | checksum=0x%016llx | clipping=%llu\n",
           music.title[0] ? music.title : path,
           stats.duration_ms / 1000.0,
           pcm_len,
           (unsigned long long)stats.checksum,
           stats.clipping_count);
    audio_engine_free_buffer(pcm);
    return 0;
}

int main(void)
{
    static const char *files[] = {
        "data/music/dark_moroder.abc",
        "data/music/perturbator_loop.abc",
        "data/music/carpenter_drive.abc",
        "data/music/advanced_dsl_demo.abc",
        "data/music/multi_fx_demo.abc",
        "data/music/neon_nightdrive.abc",
        "data/music/metro_chase.abc",
        "data/music/black_sunrise.abc",
        "data/music/machine_romance.abc",
        "data/music/hypersleep_dream.abc",
        "data/music/aurora_halo.abc"
    };
    int failures = 0;

    for (size_t i = 0; i < sizeof(files) / sizeof(files[0]); i++)
        failures += render_file(files[i]);

    return failures ? 1 : 0;
}
