use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SizedSample, Stream, StreamConfig};

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

struct PlaybackShared {
    cursor_samples: AtomicUsize,
    finished: AtomicBool,
    error: Mutex<Option<String>>,
}

impl PlaybackShared {
    fn new(start_offset: usize) -> Self {
        Self {
            cursor_samples: AtomicUsize::new(start_offset),
            finished: AtomicBool::new(false),
            error: Mutex::new(None),
        }
    }
}

#[derive(Default)]
pub struct PlaybackController {
    stream: Option<Stream>,
    state: PlaybackState,
    full_len: Option<usize>,
    shared: Option<Arc<PlaybackShared>>,
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
        let full_len = self.full_len?;
        if full_len == 0 {
            return Some(0.0);
        }
        let shared = self.shared.as_ref()?;
        let cursor = shared.cursor_samples.load(Ordering::Relaxed).min(full_len);
        Some(cursor as f32 / full_len as f32)
    }

    /// Start playback at `start_progress` (clamped to [0,1)). Progress
    /// reporting still references the full buffer length so the visible
    /// playhead stays continuous across seeks.
    pub fn start_pcm_at(&mut self, samples: &[u8], start_progress: f32) -> Result<(), String> {
        if samples.is_empty() {
            let message = "empty PCM buffer".to_string();
            self.state = PlaybackState::Error(message.clone());
            return Err(message);
        }

        let full_len = samples.len();
        let offset = start_offset_for_progress(full_len, start_progress);
        let tail = Arc::<[u8]>::from(samples[offset..].to_vec());
        let shared = Arc::new(PlaybackShared::new(offset));
        let stream = build_stream(tail, full_len, offset, Arc::clone(&shared))?;

        stream.play().map_err(|error| {
            let message = format!("could not start playback stream: {error}");
            self.state = PlaybackState::Error(message.clone());
            message
        })?;

        self.stop_runtime();
        self.stream = Some(stream);
        self.shared = Some(shared);
        self.full_len = Some(full_len);
        self.state = PlaybackState::Playing;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<bool, String> {
        let stopped = self.stream.take().is_some();
        self.stop_runtime();
        self.state = PlaybackState::Stopped;
        Ok(stopped)
    }

    pub fn poll(&mut self) -> Option<Result<(), String>> {
        if !self.is_playing() {
            return None;
        }

        let shared = Arc::clone(self.shared.as_ref()?);
        let error_message = match shared.error.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => Some("playback error state lock poisoned".to_string()),
        };
        if let Some(message) = error_message {
            self.stream = None;
            self.stop_runtime();
            self.state = PlaybackState::Error(message.clone());
            return Some(Err(message));
        }

        if shared.finished.load(Ordering::Relaxed) {
            self.stream = None;
            self.stop_runtime();
            self.state = PlaybackState::Stopped;
            return Some(Ok(()));
        }

        None
    }

    fn stop_runtime(&mut self) {
        self.stream = None;
        self.shared = None;
        self.full_len = None;
    }
}

fn start_offset_for_progress(full_len: usize, start_progress: f32) -> usize {
    let clamped = start_progress.clamp(0.0, 1.0);
    let mut offset = ((full_len as f32) * clamped) as usize;
    if offset >= full_len {
        offset = full_len.saturating_sub(1);
    }
    offset
}

fn build_stream(
    tail: Arc<[u8]>,
    full_len: usize,
    start_offset: usize,
    shared: Arc<PlaybackShared>,
) -> Result<Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no output audio device available".to_string())?;
    let output_config = device.default_output_config().map_err(|error| {
        format!("could not read default output audio configuration: {error}")
    })?;

    let sample_format = output_config.sample_format();
    let stream_config: StreamConfig = output_config.into();
    let channels = usize::from(stream_config.channels.max(1));
    let output_rate = stream_config.sample_rate.max(1);
    let step = SAMPLE_RATE_ABC as f64 / output_rate as f64;

    let shared_err = Arc::clone(&shared);
    let error_callback = move |error: cpal::StreamError| {
        if let Ok(mut guard) = shared_err.error.lock() {
            *guard = Some(format!("playback stream error: {error}"));
        }
        shared_err.finished.store(true, Ordering::Relaxed);
    };

    let source_position = Arc::new(Mutex::new(0.0_f64));

    match sample_format {
        SampleFormat::F32 => build_output_stream::<f32>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::I8 => build_output_stream::<i8>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::I16 => build_output_stream::<i16>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::I32 => build_output_stream::<i32>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::I64 => build_output_stream::<i64>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::U8 => build_output_stream::<u8>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::U16 => build_output_stream::<u16>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::U32 => build_output_stream::<u32>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::U64 => build_output_stream::<u64>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        SampleFormat::F64 => build_output_stream::<f64>(
            &device,
            &stream_config,
            channels,
            tail,
            full_len,
            start_offset,
            step,
            shared,
            source_position,
            error_callback,
        ),
        other => Err(format!(
            "unsupported playback sample format: {other:?}; add an explicit stream handler"
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_output_stream<T>(
    device: &cpal::Device,
    stream_config: &StreamConfig,
    channels: usize,
    tail: Arc<[u8]>,
    full_len: usize,
    start_offset: usize,
    step: f64,
    shared: Arc<PlaybackShared>,
    source_position: Arc<Mutex<f64>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String>
where
    T: SizedSample + cpal::FromSample<f32>,
{
    let callback_shared = Arc::clone(&shared);
    let callback_tail = Arc::clone(&tail);
    let callback_position = Arc::clone(&source_position);

    device
        .build_output_stream(
            stream_config,
            move |output: &mut [T], _| {
                write_output(
                    output,
                    channels,
                    &callback_tail,
                    full_len,
                    start_offset,
                    step,
                    &callback_shared,
                    &callback_position,
                );
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("could not create playback stream: {error}"))
}

#[allow(clippy::too_many_arguments)]
fn write_output<T>(
    output: &mut [T],
    channels: usize,
    tail: &[u8],
    full_len: usize,
    start_offset: usize,
    step: f64,
    shared: &PlaybackShared,
    source_position: &Arc<Mutex<f64>>,
) where
    T: SizedSample + cpal::FromSample<f32>,
{
    let mut position_guard = match source_position.lock() {
        Ok(guard) => guard,
        Err(_) => {
            shared.finished.store(true, Ordering::Relaxed);
            shared.cursor_samples.store(full_len, Ordering::Relaxed);
            output.fill(T::from_sample(0.0_f32));
            return;
        }
    };

    let mut pos = *position_guard;
    let frame_count = output.len() / channels;
    let mut finished = false;

    for frame in 0..frame_count {
        let source_index = pos.floor() as usize;
        let sample_f32 = if source_index < tail.len() {
            (tail[source_index] as f32 - 128.0) / 128.0
        } else {
            finished = true;
            0.0
        };

        for channel in 0..channels {
            output[frame * channels + channel] = T::from_sample(sample_f32);
        }

        pos += step;
    }

    let absolute = start_offset + (pos.floor() as usize).min(tail.len());
    shared
        .cursor_samples
        .store(absolute.min(full_len), Ordering::Relaxed);

    if absolute >= full_len {
        finished = true;
    }
    if finished {
        shared.finished.store(true, Ordering::Relaxed);
    }

    *position_guard = pos;
}

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
    fn progress_requires_active_runtime() {
        let playback = PlaybackController::default();
        assert_eq!(playback.progress(), None);
    }

    #[test]
    fn stop_returns_false_without_active_stream() {
        let mut playback = PlaybackController::default();
        let stopped = playback.stop().expect("stop should succeed");
        assert!(!stopped, "stop should report false with no active stream");
    }

    #[test]
    fn progress_uses_cursor_ratio() {
        let mut playback = PlaybackController::default();
        playback.full_len = Some(1000);
        let shared = Arc::new(PlaybackShared::new(0));
        shared.cursor_samples.store(250, Ordering::Relaxed);
        playback.shared = Some(shared);

        let p = playback.progress().expect("progress should be available");
        assert!((p - 0.25).abs() < f32::EPSILON, "expected 0.25, got {p}");
    }

    #[test]
    fn start_offset_clamps_to_last_sample() {
        let offset = start_offset_for_progress(100, 1.5);
        assert_eq!(offset, 99);
    }
}
