/*
 * miniaudio_playback.c — In-process PCM playback via miniaudio.
 *
 * This translation unit defines MINIAUDIO_IMPLEMENTATION so it must be
 * compiled exactly once.  All other files that need playback should include
 * only miniaudio_playback.h.
 */

#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
#include "miniaudio_playback.h"

#include <stdlib.h>
#include <string.h>

/* ── Internal state ──────────────────────────────────────────────────────── */

struct MaPlaybackHandle {
    ma_device   device;
    ma_mutex    mutex;
    int         device_initialized;

    unsigned char *pcm_data;  /* heap copy of the caller's buffer         */
    size_t         pcm_len;   /* total bytes in pcm_data                  */
    size_t         pcm_offset;/* bytes consumed so far                    */
    unsigned int   sample_rate;
    int            loop;      /* 1 = restart when buffer exhausted        */
    int            playing;   /* 1 = device is feeding data               */
};

/* ── Data callback (runs on miniaudio's audio thread) ────────────────────── */

static void data_callback(ma_device *pDevice, void *pOutput,
                          const void *pInput, ma_uint32 frameCount)
{
    MaPlaybackHandle *h = (MaPlaybackHandle *)pDevice->pUserData;
    unsigned char    *out = (unsigned char *)pOutput;
    ma_uint32         frames_written = 0;

    (void)pInput;

    ma_mutex_lock(&h->mutex);

    while (frames_written < frameCount) {
        size_t    remaining = h->pcm_len - h->pcm_offset;
        ma_uint32 want      = frameCount - frames_written;
        size_t    to_copy   = (want < remaining) ? (size_t)want : remaining;

        if (to_copy > 0) {
            memcpy(out + frames_written,
                   h->pcm_data + h->pcm_offset,
                   to_copy);
            h->pcm_offset  += to_copy;
            frames_written += (ma_uint32)to_copy;
        }

        if (h->pcm_offset >= h->pcm_len) {
            if (h->loop) {
                /* Wrap around for looping playback. */
                h->pcm_offset = 0;
            } else {
                /* Fill any remaining output with silence then mark done. */
                size_t tail = (size_t)(frameCount - frames_written);
                if (tail > 0)
                    memset(out + frames_written, 128, tail);
                frames_written = frameCount;   /* exit the loop */
                h->playing = 0;
                break;
            }
        }
    }

    ma_mutex_unlock(&h->mutex);
}

/* ── Public API ──────────────────────────────────────────────────────────── */

MaPlaybackHandle *ma_pb_create(void)
{
    MaPlaybackHandle *h = (MaPlaybackHandle *)calloc(1, sizeof(*h));
    if (!h)
        return NULL;

    if (ma_mutex_init(&h->mutex) != MA_SUCCESS) {
        free(h);
        return NULL;
    }

    return h;
}

void ma_pb_destroy(MaPlaybackHandle *h)
{
    if (!h)
        return;

    ma_pb_stop(h);

    if (h->device_initialized) {
        ma_device_uninit(&h->device);
        h->device_initialized = 0;
    }

    free(h->pcm_data);
    h->pcm_data = NULL;

    ma_mutex_uninit(&h->mutex);
    free(h);
}

int ma_pb_start(MaPlaybackHandle *h,
                const unsigned char *pcm, size_t len,
                unsigned int sample_rate, int loop)
{
    if (!h || !pcm || len == 0)
        return -1;

    /* Stop any running playback and teardown the old device when the sample
     * rate changes — miniaudio devices are bound to a fixed rate at init. */
    if (h->device_initialized) {
        ma_device_stop(&h->device);
        if (h->sample_rate != sample_rate) {
            ma_device_uninit(&h->device);
            h->device_initialized = 0;
        }
    }

    /* (Re-)allocate the PCM buffer. */
    if (h->pcm_len != len) {
        free(h->pcm_data);
        h->pcm_data = (unsigned char *)malloc(len);
        if (!h->pcm_data) {
            h->pcm_len = 0;
            return -1;
        }
    }
    memcpy(h->pcm_data, pcm, len);
    h->pcm_len    = len;
    h->pcm_offset = 0;
    h->sample_rate = sample_rate;
    h->loop       = loop;
    h->playing    = 1;

    /* Initialise the device once (or after a sample-rate change). */
    if (!h->device_initialized) {
        ma_device_config cfg = ma_device_config_init(ma_device_type_playback);
        cfg.playback.format   = ma_format_u8;
        cfg.playback.channels = 1;
        cfg.sampleRate        = sample_rate;
        cfg.dataCallback      = data_callback;
        cfg.pUserData         = h;

        if (ma_device_init(NULL, &cfg, &h->device) != MA_SUCCESS) {
            free(h->pcm_data);
            h->pcm_data = NULL;
            h->pcm_len  = 0;
            h->playing  = 0;
            return -1;
        }
        h->device_initialized = 1;
    }

    if (ma_device_start(&h->device) != MA_SUCCESS) {
        ma_device_uninit(&h->device);
        h->device_initialized = 0;
        free(h->pcm_data);
        h->pcm_data = NULL;
        h->pcm_len  = 0;
        h->playing  = 0;
        return -1;
    }

    return 0;
}

int ma_pb_stop(MaPlaybackHandle *h)
{
    if (!h)
        return 0;

    int was_playing;

    ma_mutex_lock(&h->mutex);
    was_playing = h->playing;
    h->playing  = 0;
    ma_mutex_unlock(&h->mutex);

    if (h->device_initialized &&
        ma_device_get_state(&h->device) == ma_device_state_started)
        ma_device_stop(&h->device);

    return was_playing;
}

int ma_pb_is_active(MaPlaybackHandle *h)
{
    if (!h || !h->device_initialized)
        return 0;
    if (ma_device_get_state(&h->device) != ma_device_state_started)
        return 0;

    ma_mutex_lock(&h->mutex);
    int active = h->playing;
    ma_mutex_unlock(&h->mutex);
    return active;
}

float ma_pb_progress(MaPlaybackHandle *h)
{
    if (!h || h->pcm_len == 0)
        return -1.0f;

    ma_mutex_lock(&h->mutex);
    float p = (float)h->pcm_offset / (float)h->pcm_len;
    ma_mutex_unlock(&h->mutex);
    return p;
}
