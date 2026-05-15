use std::path::{Path, PathBuf};

use crate::ffi;

const SHOWCASE_DEMOS: [&str; 10] = [
    "dark_moroder",
    "neon_nightdrive",
    "metro_chase",
    "black_sunrise",
    "machine_romance",
    "hypersleep_dream",
    "perturbator_loop",
    "carpenter_drive",
    "advanced_dsl_demo",
    "multi_fx_demo",
];

#[derive(Clone, Debug)]
pub struct PatternBlock {
    pub label: String,
    pub length: usize,
    pub start_step: usize,
}

#[derive(Clone, Debug)]
pub struct TrackOverview {
    pub name: String,
    pub instrument: String,
    pub activity: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct DemoOverview {
    pub title: String,
    pub bpm: i32,
    pub swing_pct: i32,
    pub total_steps: usize,
    pub arrangement: Vec<PatternBlock>,
    pub tracks: Vec<TrackOverview>,
    pub hidden_track_count: usize,
}

#[derive(Clone, Debug)]
pub struct DemoEntry {
    pub key: String,
    pub path: PathBuf,
    pub overview: Option<DemoOverview>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RenderState {
    pub demo_key: String,
    pub samples: Vec<u8>,
    pub stats: Option<AudioRenderStats>,
}

pub type AudioRenderStats = ffi::AudioRenderStats;

#[derive(Default)]
pub struct GuiAudioEngine;

impl GuiAudioEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn demo_catalog(&self) -> Vec<DemoEntry> {
        let root = repository_root();

        SHOWCASE_DEMOS
            .into_iter()
            .map(|name| {
                let path = root.join("data").join("music").join(format!("{name}.abc"));
                match ffi::load_demo_overview(&path) {
                    Ok(overview) => DemoEntry {
                        key: name.to_string(),
                        path,
                        overview: Some(overview),
                        error: None,
                    },
                    Err(error) => DemoEntry {
                        key: name.to_string(),
                        path,
                        overview: None,
                        error: Some(error),
                    },
                }
            })
            .collect()
    }

    pub fn render_demo(&self, demo_key: &str, path: &Path) -> Result<RenderState, String> {
        let samples = ffi::render_abc_file(path)?;
        Ok(RenderState {
            demo_key: demo_key.to_string(),
            samples,
            stats: ffi::get_render_stats(),
        })
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_catalog_loads() {
        let engine = GuiAudioEngine::new();
        let catalog = engine.demo_catalog();

        assert!(!catalog.is_empty(), "demo catalog should not be empty");
        assert!(
            catalog.iter().any(|entry| entry.key == "dark_moroder"),
            "catalog should include dark_moroder"
        );
    }

    #[test]
    fn invalid_path_fails_cleanly() {
        let engine = GuiAudioEngine::new();
        let invalid = PathBuf::from("/definitely/not/a/real/memdeck-demo.abc");

        let result = engine.render_demo("missing_demo", &invalid);
        assert!(result.is_err(), "missing file should return an error");
        let error = result.err().unwrap_or_default();
        assert!(
            error.contains("missing demo file"),
            "error should mention missing demo file, got: {error}"
        );
    }

    #[test]
    fn render_valid_demo_succeeds() {
        let engine = GuiAudioEngine::new();
        let demo = engine
            .demo_catalog()
            .into_iter()
            .find(|entry| entry.key == "dark_moroder" && entry.overview.is_some())
            .expect("dark_moroder demo should be available");

        let render = engine
            .render_demo(&demo.key, &demo.path)
            .expect("valid demo should render");
        assert!(!render.samples.is_empty(), "rendered sample buffer should be non-empty");
    }

    #[test]
    fn repeated_render_does_not_crash() {
        let engine = GuiAudioEngine::new();
        let demo = engine
            .demo_catalog()
            .into_iter()
            .find(|entry| entry.key == "dark_moroder" && entry.overview.is_some())
            .expect("dark_moroder demo should be available");

        for _ in 0..3 {
            let render = engine
                .render_demo(&demo.key, &demo.path)
                .expect("repeated render should succeed");
            assert!(!render.samples.is_empty(), "rendered sample buffer should be non-empty");
        }
    }
}
