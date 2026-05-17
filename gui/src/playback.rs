use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::ffi::SAMPLE_RATE_ABC;

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

#[derive(Default)]
pub struct PlaybackController {
    child: Option<Child>,
    temp_path: Option<PathBuf>,
    state: PlaybackState,
    started_at: Option<Instant>,
    full_duration: Option<Duration>,
    start_progress: f32,
}

impl PlaybackController {
    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlaybackState::Playing)
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Live progress in [0,1] across the full PCM buffer the user originally
    /// asked to play, even if playback was started from an offset.
    pub fn progress(&self) -> Option<f32> {
        let elapsed = self.started_at?.elapsed();
        let duration = self.full_duration?;
        if duration.is_zero() {
            return Some(self.start_progress.clamp(0.0, 1.0));
        }

        let elapsed_fraction = elapsed.as_secs_f32() / duration.as_secs_f32();
        Some((self.start_progress + elapsed_fraction).clamp(0.0, 1.0))
    }

    /// Start playback at `start_progress` (clamped to [0,1)). The audio child
    /// only receives the tail of the PCM buffer; progress reporting still
    /// references the full buffer length so the visible playhead stays
    /// continuous across seeks.
    pub fn start_pcm_at(&mut self, samples: &[u8], start_progress: f32) -> Result<(), String> {
        if samples.is_empty() {
            let message = "empty PCM buffer".to_string();
            self.state = PlaybackState::Error(message.clone());
            return Err(message);
        }

        let full_len = samples.len();
        let clamped = start_progress.clamp(0.0, 1.0);
        let mut offset = ((full_len as f32) * clamped) as usize;
        // Leave at least one sample so aplay has something to play.
        if offset >= full_len {
            offset = full_len.saturating_sub(1);
        }
        let effective_progress = if full_len == 0 {
            0.0
        } else {
            offset as f32 / full_len as f32
        };
        let slice = &samples[offset..];

        let path = playback_temp_path();
        write_wav_u8_mono(&path, slice, SAMPLE_RATE_ABC as u32)
            .map_err(|err| format!("could not write playback buffer: {err}"))?;

        if let Err(err) = self.start_wav(&path) {
            let _ = fs::remove_file(&path);
            return Err(err);
        }

        self.temp_path = Some(path);
        self.full_duration = Some(Duration::from_secs_f32(
            full_len as f32 / SAMPLE_RATE_ABC as f32,
        ));
        self.start_progress = effective_progress;
        Ok(())
    }

    pub fn start_wav(&mut self, path: &Path) -> Result<(), String> {
        self.stop()
            .map_err(|err| format!("could not reset playback: {err}"))?;

        let mut command = playback_command(path)?;
        let child = command.spawn().map_err(|err| {
            let message = format!("could not start playback command: {err}");
            self.state = PlaybackState::Error(message.clone());
            message
        })?;

        self.child = Some(child);
        self.started_at = Some(Instant::now());
        self.state = PlaybackState::Playing;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<bool, String> {
        let mut stopped = false;

        if let Some(mut child) = self.child.take() {
            stopped = true;
            if let Err(err) = child.kill() {
                if err.kind() != io::ErrorKind::InvalidInput {
                    self.cleanup_runtime_state();
                    self.state = PlaybackState::Error(err.to_string());
                    return Err(err.to_string());
                }
            }
            let _ = child.wait();
        }

        self.cleanup_runtime_state();
        self.state = PlaybackState::Stopped;
        Ok(stopped)
    }

    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let status = match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => status,
                Ok(None) => return None,
                Err(err) => {
                    self.child = None;
                    self.cleanup_runtime_state();
                    let message = format!("could not poll playback process: {err}");
                    self.state = PlaybackState::Error(message.clone());
                    return Some(Err(message));
                }
            },
            None => return None,
        };

        self.child = None;
        self.cleanup_runtime_state();

        if status.success() {
            self.state = PlaybackState::Stopped;
            Some(Ok(()))
        } else {
            let message = format!("playback exited with status {status}");
            self.state = PlaybackState::Error(message.clone());
            Some(Err(message))
        }
    }

    fn cleanup_runtime_state(&mut self) {
        self.started_at = None;
        self.full_duration = None;
        self.start_progress = 0.0;
        self.cleanup_temp_file();
    }

    fn cleanup_temp_file(&mut self) {
        if let Some(path) = self.temp_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn playback_temp_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    env::temp_dir().join(format!(
        "memdeck-gui-{}-{timestamp}.wav",
        std::process::id()
    ))
}

fn playback_command(path: &Path) -> Result<Command, String> {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("afplay");
        command.arg(path);
        return Ok(command);
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            "& { \
                $p = [System.IO.Path]::GetFullPath($args[0]); \
                $player = New-Object Media.SoundPlayer; \
                $player.SoundLocation = $p; \
                $player.Load(); \
                $player.PlaySync(); \
            }",
        ]);
        command.arg(path);
        return Ok(command);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let mut command = Command::new("aplay");
        command.arg("-q").arg(path);
        Ok(command)
    }
}

fn write_wav_u8_mono(path: &Path, pcm: &[u8], sample_rate: u32) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let data_size = pcm.len() as u32;
    let riff_size = 36u32 + data_size;

    file.write_all(b"RIFF")?;
    file.write_all(&riff_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&8u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    file.write_all(pcm)?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn progress_requires_active_timing() {
        let playback = PlaybackController::default();
        assert_eq!(playback.progress(), None);
    }

    #[test]
    fn stop_cleans_up_temp_file() {
        let mut playback = PlaybackController::default();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_path = std::env::temp_dir().join(format!(
            "memdeck-playback-test-{unique}-{}.wav",
            std::process::id()
        ));
        fs::write(&temp_path, [0_u8, 1, 2]).expect("should write temp wav fixture");
        playback.temp_path = Some(temp_path.clone());

        let stopped = playback.stop().expect("stop should succeed");
        assert!(!stopped, "stop should report false with no child process");
        assert!(
            !temp_path.exists(),
            "stop should always remove owned temporary wav files"
        );
    }

    #[test]
    fn start_progress_clamps_and_records_offset() {
        let mut playback = PlaybackController::default();
        // Out-of-range progress is clamped; full_duration tracks the *whole*
        // buffer, not the trimmed tail, so the visible playhead can interpolate
        // from the seek point all the way to 1.0.
        playback.full_duration = Some(Duration::from_secs(10));
        playback.start_progress = 1.5;
        playback.started_at = Some(Instant::now());

        let p = playback.progress().expect("progress should be available");
        assert!(p >= 1.0 - f32::EPSILON, "progress should clamp to 1.0, got {p}");
    }

}
