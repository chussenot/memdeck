#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "../src/memdeck.h"

static uint64_t fnv1a64(const unsigned char *data, int len)
{
    uint64_t hash = 1469598103934665603ull;
    for (int i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= 1099511628211ull;
    }
    return hash;
}

static int render_file(const char *path)
{
    AbcMusic music;
    unsigned char *pcm = NULL;
    int pcm_len = 0;
    int minv = 255;
    int maxv = 0;
    int peak = 0;
    int clipping = 0;
    int run = 1;
    uint64_t checksum;

    if (abc_load(path, &music) != 0) {
        printf("%s | parse=FAIL\n", path);
        return 1;
    }
    pcm = abc_generate_pcm(&music, &pcm_len);
    if (!pcm || pcm_len <= 0) {
        free(pcm);
        printf("%s | render=FAIL\n", path);
        return 1;
    }

    checksum = fnv1a64(pcm, pcm_len);
    for (int i = 0; i < pcm_len; i++) {
        int s = (int)pcm[i];
        int a = s - 128;
        if (a < 0) a = -a;
        if (s < minv) minv = s;
        if (s > maxv) maxv = s;
        if (a > peak) peak = a;
        if (i > 0 && pcm[i] == pcm[i - 1]) run++;
        else run = 1;
        if (run >= 4 && (s == 0 || s == 255))
            clipping++;
    }

    printf("%s | duration=%.3fs | pcm_len=%d | checksum=0x%016llx | peak=%d | min=%d | max=%d | clipping=%d\n",
           path, pcm_len / 22050.0, pcm_len, (unsigned long long)checksum,
           peak, minv, maxv, clipping);
    free(pcm);
    return 0;
}

int main(void)
{
    static const char *files[] = {
        "data/music/dark_moroder.abc",
        "data/music/perturbator_loop.abc",
        "data/music/carpenter_drive.abc",
        "data/music/advanced_dsl_demo.abc",
        "data/music/multi_fx_demo.abc"
    };
    int failures = 0;

    for (size_t i = 0; i < sizeof(files) / sizeof(files[0]); i++)
        failures += render_file(files[i]);

    return failures ? 1 : 0;
}
