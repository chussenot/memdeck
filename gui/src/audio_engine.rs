use std::path::Path;

use crate::ffi;

#[derive(Clone, Debug)]
pub struct RenderState {
    pub samples: Vec<u8>,
    pub stats: Option<ffi::AudioRenderStats>,
}

#[derive(Default)]
pub struct GuiAudioEngine;

impl GuiAudioEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn render_builtin_menu(&self) -> Result<RenderState, String> {
        let samples = ffi::render_builtin_menu()?;
        Ok(RenderState {
            samples,
            stats: ffi::get_render_stats(),
        })
    }

    pub fn render_abc_file(&self, path: &Path) -> Result<RenderState, String> {
        let samples = ffi::render_abc_file(path)?;
        Ok(RenderState {
            samples,
            stats: ffi::get_render_stats(),
        })
    }

    pub fn get_render_stats(&self) -> Option<ffi::AudioRenderStats> {
        ffi::get_render_stats()
    }
}
