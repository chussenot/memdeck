use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::slice;
use std::sync::{LazyLock, Mutex};

pub const SAMPLE_RATE_ABC: c_int = 22_050;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioRenderStats {
    pub sample_count: u64,
    pub duration_ms: f64,
    pub min_sample: c_int,
    pub max_sample: c_int,
    pub peak: c_int,
    pub clipping_count: u64,
    pub checksum: u64,
    pub render_time_ms: f64,
}

unsafe extern "C" {
    fn audio_engine_render_builtin_menu(
        sample_rate: c_int,
        out_len: *mut c_int,
        out_stats: *mut AudioRenderStats,
    ) -> *mut u8;

    fn audio_engine_render_abc_file(
        path: *const c_char,
        sample_rate: c_int,
        out_len: *mut c_int,
        out_stats: *mut AudioRenderStats,
    ) -> *mut u8;

    fn audio_engine_free_buffer(buffer: *mut u8);
}

static LAST_RENDER_STATS: LazyLock<Mutex<Option<AudioRenderStats>>> =
    LazyLock::new(|| Mutex::new(None));

fn copy_and_release_buffer(buffer: *mut u8, len: c_int) -> Result<Vec<u8>, String> {
    if buffer.is_null() || len <= 0 {
        return Err("audio render failed or returned empty PCM buffer".to_string());
    }

    let samples = unsafe {
        let slice = slice::from_raw_parts(buffer as *const u8, len as usize);
        slice.to_vec()
    };

    free_buffer(buffer);
    Ok(samples)
}

pub fn render_builtin_menu() -> Result<Vec<u8>, String> {
    let mut pcm_len = 0;
    let mut stats = AudioRenderStats::default();

    let buffer = unsafe { audio_engine_render_builtin_menu(SAMPLE_RATE_ABC, &mut pcm_len, &mut stats) };
    let samples = copy_and_release_buffer(buffer, pcm_len)?;

    if let Ok(mut last_stats) = LAST_RENDER_STATS.lock() {
        *last_stats = Some(stats);
    }

    Ok(samples)
}

pub fn render_abc_file(path: &Path) -> Result<Vec<u8>, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "demo path is not valid UTF-8".to_string())?;
    let c_path = CString::new(path_str).map_err(|_| "demo path contains NUL byte".to_string())?;

    let mut pcm_len = 0;
    let mut stats = AudioRenderStats::default();

    let buffer = unsafe {
        audio_engine_render_abc_file(c_path.as_ptr(), SAMPLE_RATE_ABC, &mut pcm_len, &mut stats)
    };
    let samples = copy_and_release_buffer(buffer, pcm_len)?;

    if let Ok(mut last_stats) = LAST_RENDER_STATS.lock() {
        *last_stats = Some(stats);
    }

    Ok(samples)
}

pub fn free_buffer(buffer: *mut u8) {
    if !buffer.is_null() {
        unsafe {
            audio_engine_free_buffer(buffer);
        }
    }
}

pub fn get_render_stats() -> Option<AudioRenderStats> {
    LAST_RENDER_STATS.lock().ok().and_then(|stats| *stats)
}
