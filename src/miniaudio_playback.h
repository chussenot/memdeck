/*
 * miniaudio_playback.h — Thin wrapper around miniaudio for in-process PCM
 * playback used by both the TUI (sound.c) and the GUI (via Rust FFI).
 *
 * Features
 * --------
 *  - Cross-platform: uses miniaudio's device abstraction (CoreAudio on macOS,
 *    WASAPI on Windows, ALSA/PulseAudio loaded at runtime on Linux).
 *  - No external tool dependency: replaces aplay / afplay / PowerShell.
 *  - Raw u8 mono PCM playback (the format produced by the audio engine).
 *  - Optional looping for background music.
 */

#ifndef MINIAUDIO_PLAYBACK_H
#define MINIAUDIO_PLAYBACK_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle — allocated and owned by the caller via ma_pb_create(). */
typedef struct MaPlaybackHandle MaPlaybackHandle;

/* Allocate a new handle.  Returns NULL on allocation failure. */
MaPlaybackHandle *ma_pb_create(void);

/*
 * Free a handle.  Stops any active playback and releases all resources.
 * Passing NULL is safe.
 */
void ma_pb_destroy(MaPlaybackHandle *h);

/*
 * Start playing raw unsigned-8-bit mono PCM sampled at `sample_rate` Hz.
 * The PCM data is copied internally; the caller may free `pcm` afterwards.
 * If `loop` is non-zero the buffer restarts from the beginning when consumed.
 * Returns 0 on success, -1 on error.
 */
int ma_pb_start(MaPlaybackHandle *h,
                const unsigned char *pcm, size_t len,
                unsigned int sample_rate, int loop);

/*
 * Stop playback immediately.
 * Returns 1 if playback was active at the time of the call, 0 otherwise.
 */
int ma_pb_stop(MaPlaybackHandle *h);

/*
 * Returns 1 if the device is running and the PCM buffer has not been fully
 * consumed (or loop is enabled), 0 otherwise.
 */
int ma_pb_is_active(MaPlaybackHandle *h);

/*
 * Progress through the current PCM buffer in [0, 1].
 * Returns -1.0f if no playback has been started.
 */
float ma_pb_progress(MaPlaybackHandle *h);

#ifdef __cplusplus
}
#endif

#endif /* MINIAUDIO_PLAYBACK_H */
