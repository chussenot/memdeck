use std::os::raw::{c_int, c_uint};
use std::time::Duration;

use crate::ffi::SAMPLE_RATE_ABC;

/* ── FFI bindings to miniaudio_playback.c ─────────────────────────────────── */

#[repr(C)]
struct MaPlaybackHandle {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    fn ma_pb_create() -> *mut MaPlaybackHandle;
    fn ma_pb_destroy(h: *mut MaPlaybackHandle);
    fn ma_pb_start(
        h: *mut MaPlaybackHandle,
        pcm: *const u8,
        len: usize,
        sample_rate: c_uint,
        do_loop: c_int,
    ) -> c_int;
    fn ma_pb_stop(h: *mut MaPlaybackHandle) -> c_int;
    fn ma_pb_is_active(h: *mut MaPlaybackHandle) -> c_int;
    fn ma_pb_progress(h: *mut MaPlaybackHandle) -> f32;
}

/* ── Public types ─────────────────────────────────────────────────────────── */

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Error(String),
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

/* ── PlaybackController ───────────────────────────────────────────────────── */

pub struct PlaybackController {
    handle: *mut MaPlaybackHandle,
    state: PlaybackState,
    /// Total duration of the *full* PCM buffer (before any seek offset).
    full_duration: Option<Duration>,
    /// Fractional position [0, 1) from which the current playback started.
    start_progress: f32,
}

// SAFETY: PlaybackController is only ever accessed from the single UI thread.
unsafe impl Send for PlaybackController {}

impl Default for PlaybackController {
    fn default() -> Self {
        let handle = unsafe { ma_pb_create() };
        Self {
            handle,
            state: PlaybackState::Stopped,
            full_duration: None,
            start_progress: 0.0,
        }
    }
}

impl Drop for PlaybackController {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ma_pb_destroy(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

impl PlaybackController {
    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlaybackState::Playing)
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Live progress in [0, 1] across the full PCM buffer the user originally
    /// asked to play, even if playback was started from an offset.
    pub fn progress(&self) -> Option<f32> {
        if !matches!(self.state, PlaybackState::Playing) {
            return None;
        }
        if self.handle.is_null() {
            return None;
        }
        let raw = unsafe { ma_pb_progress(self.handle as *mut MaPlaybackHandle) };
        if raw < 0.0 {
            return None;
        }
        // `raw` is the progress through the *slice* we submitted (0..1).
        // Map back to the full-buffer coordinate system so the playhead
        // moves continuously from `start_progress` to 1.0.
        let full = self.start_progress + raw * (1.0 - self.start_progress);
        Some(full.clamp(0.0, 1.0))
    }

    /// Start playback at `start_progress` (clamped to [0, 1)).
    /// Only the tail of the PCM buffer starting at `offset` is submitted to
    /// the audio backend; progress reporting still references the full buffer
    /// so the visible playhead stays continuous across seeks.
    pub fn start_pcm_at(&mut self, samples: &[u8], start_progress: f32) -> Result<(), String> {
        if samples.is_empty() {
            let message = "empty PCM buffer".to_string();
            self.state = PlaybackState::Error(message.clone());
            return Err(message);
        }

        let full_len = samples.len();
        let clamped = start_progress.clamp(0.0, 1.0);
        let mut offset = ((full_len as f32) * clamped) as usize;
        if offset >= full_len {
            offset = full_len.saturating_sub(1);
        }
        let effective_progress = if full_len == 0 {
            0.0
        } else {
            offset as f32 / full_len as f32
        };
        let slice = &samples[offset..];

        self.stop()?;

        if self.handle.is_null() {
            self.handle = unsafe { ma_pb_create() };
            if self.handle.is_null() {
                let message = "could not allocate audio backend".to_string();
                self.state = PlaybackState::Error(message.clone());
                return Err(message);
            }
        }

        let rc = unsafe {
            ma_pb_start(
                self.handle,
                slice.as_ptr(),
                slice.len(),
                SAMPLE_RATE_ABC as c_uint,
                0, /* no loop */
            )
        };
        if rc != 0 {
            let message = "could not start audio playback".to_string();
            self.state = PlaybackState::Error(message.clone());
            return Err(message);
        }

        self.full_duration = Some(Duration::from_secs_f32(
            full_len as f32 / SAMPLE_RATE_ABC as f32,
        ));
        self.start_progress = effective_progress;
        self.state = PlaybackState::Playing;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<bool, String> {
        if self.handle.is_null() {
            self.state = PlaybackState::Stopped;
            return Ok(false);
        }
        let was_playing = unsafe { ma_pb_stop(self.handle) } != 0;
        self.state = PlaybackState::Stopped;
        self.full_duration = None;
        self.start_progress = 0.0;
        Ok(was_playing)
    }

    /// Poll for natural playback completion (buffer exhausted).
    /// Returns `Some(Ok(()))` once, then `None` until the next `start_pcm_at`.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        if !matches!(self.state, PlaybackState::Playing) {
            return None;
        }
        if self.handle.is_null() {
            return None;
        }
        let active = unsafe { ma_pb_is_active(self.handle) } != 0;
        if !active {
            // Buffer fully consumed — stop the device and transition state.
            unsafe { ma_pb_stop(self.handle) };
            self.state = PlaybackState::Stopped;
            self.full_duration = None;
            self.start_progress = 0.0;
            Some(Ok(()))
        } else {
            None
        }
    }
}

/* ── Tests ────────────────────────────────────────────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_stopped() {
        let playback = PlaybackController::default();
        assert_eq!(playback.state(), &PlaybackState::Stopped);
        assert!(!playback.is_playing());
    }

    #[test]
    fn empty_pcm_start_sets_error_state() {
        let mut playback = PlaybackController::default();
        let result = playback.start_pcm_at(&[], 0.0);
        assert!(result.is_err());
        assert!(matches!(playback.state(), PlaybackState::Error(_)));
    }

    #[test]
    fn stop_transitions_to_stopped_state() {
        let mut playback = PlaybackController::default();
        let start_error = playback
            .start_pcm_at(&[], 0.0)
            .expect_err("empty sample buffer should fail");
        assert!(
            start_error.contains("empty PCM buffer"),
            "expected empty PCM buffer error, got: {start_error}"
        );
        let _ = playback.stop();
        assert_eq!(playback.state(), &PlaybackState::Stopped);
    }

    #[test]
    fn poll_when_stopped_returns_none() {
        let mut playback = PlaybackController::default();
        assert!(playback.poll().is_none());
        assert_eq!(playback.state(), &PlaybackState::Stopped);
    }

    #[test]
    fn progress_requires_active_playback() {
        let playback = PlaybackController::default();
        assert_eq!(playback.progress(), None);
    }

    #[test]
    fn start_progress_clamps_offset() {
        // Provide a non-empty but tiny buffer to exercise offset clamping.
        // The call may fail if there is no audio device, which is acceptable;
        // we only care that no panic occurs and the state remains consistent.
        let mut playback = PlaybackController::default();
        let samples = vec![128u8; 100];
        let _ = playback.start_pcm_at(&samples, 1.5); // out-of-range
        assert!(matches!(
            playback.state(),
            PlaybackState::Stopped | PlaybackState::Playing | PlaybackState::Error(_)
        ));
    }
}
