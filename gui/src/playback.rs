use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ffi::SAMPLE_RATE_ABC;

#[derive(Default)]
pub struct PlaybackController {
    child: Option<Child>,
    temp_path: Option<PathBuf>,
}

impl PlaybackController {
    pub fn is_playing(&self) -> bool {
        self.child.is_some()
    }

    pub fn start(&mut self, samples: &[u8]) -> Result<(), String> {
        self.stop().map_err(|err| format!("could not reset playback: {err}"))?;

        let path = playback_temp_path();
        write_wav_u8_mono(&path, samples, SAMPLE_RATE_ABC as u32)
            .map_err(|err| format!("could not write playback buffer: {err}"))?;

        let mut command = playback_command(&path)?;
        let child = command
            .spawn()
            .map_err(|err| format!("could not start playback command: {err}"))?;

        self.temp_path = Some(path);
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<bool, String> {
        let mut stopped = false;

        if let Some(mut child) = self.child.take() {
            stopped = true;
            if let Err(err) = child.kill() {
                if err.kind() != io::ErrorKind::InvalidInput {
                    self.cleanup_temp_file();
                    return Err(err.to_string());
                }
            }
            let _ = child.wait();
        }

        self.cleanup_temp_file();
        Ok(stopped)
    }

    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let status = match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => status,
                Ok(None) => return None,
                Err(err) => {
                    self.child = None;
                    self.cleanup_temp_file();
                    return Some(Err(format!("could not poll playback process: {err}")));
                }
            },
            None => return None,
        };

        self.child = None;
        self.cleanup_temp_file();

        if status.success() {
            Some(Ok(()))
        } else {
            Some(Err(format!("playback exited with status {status}")))
        }
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
        let wav_path = path.display().to_string().replace('\\', "\\\\");
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            &format!(
                "(New-Object Media.SoundPlayer '{wav_path}').PlaySync()"
            ),
        ]);
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
