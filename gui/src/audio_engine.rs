use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ffi;

// Showcase demos are discovered at runtime from data/music/*.abc.
// Files whose stem starts with "menu" are skipped (those are UI sound effects,
// not showcase songs).
fn scan_showcase_demos(root: &Path) -> Vec<String> {
    let dir = root.join("data").join("music");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("abc") {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            if stem.starts_with("menu") {
                return None;
            }
            Some(stem)
        })
        .collect();
    keys.sort();
    keys
}

#[derive(Clone, Debug)]
pub struct PatternBlock {
    pub label: String,
    pub length: usize,
    pub start_step: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepState {
    pub active: bool,
    pub accent: bool,
    pub fx_trigger: bool,
}

#[derive(Clone, Debug)]
pub struct TrackOverview {
    pub name: String,
    pub instrument: String,
    pub preset: String,
    pub waveform: String,
    pub amplitude: i32,
    pub duty_cycle: i32,
    pub attack_ms: i32,
    pub decay_ms: i32,
    pub sustain_level: i32,
    pub release_ms: i32,
    pub gate_percent: i32,
    pub vibrato_cents: i32,
    pub vibrato_rate: i32,
    pub glide_ms: i32,
    pub detune_cents: i32,
    pub fx_bus: usize,
    pub activity: Vec<StepState>,
}

#[derive(Clone, Debug)]
pub struct FxBusOverview {
    pub bus_index: usize,
    pub enabled: bool,
    pub delay_steps: i32,
    pub delay_feedback: i32,
    pub delay_mix: i32,
    pub drive_amount: i32,
    pub lowpass_amount: i32,
    pub sidechain_amount: i32,
    pub sidechain_release_ms: i32,
    pub mix_percent: i32,
}

#[derive(Clone, Debug)]
pub struct DemoOverview {
    pub title: String,
    pub bpm: i32,
    pub swing_pct: i32,
    pub steps_per_beat: usize,
    pub total_steps: usize,
    pub arrangement: Vec<PatternBlock>,
    pub tracks: Vec<TrackOverview>,
    pub fx_buses: Vec<FxBusOverview>,
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
    pub samples: Arc<[u8]>,
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
        scan_showcase_demos(&root)
            .into_iter()
            .map(|name| {
                let path = root.join("data").join("music").join(format!("{name}.abc"));
                match ffi::load_demo_overview(&path) {
                    Ok(overview) => DemoEntry {
                        key: name,
                        path,
                        overview: Some(overview),
                        error: None,
                    },
                    Err(error) => DemoEntry {
                        key: name,
                        path,
                        overview: None,
                        error: Some(error),
                    },
                }
            })
            .collect()
    }

    pub fn render_demo(&self, demo_key: &str, path: &Path) -> Result<RenderState, String> {
        let samples = Arc::<[u8]>::from(ffi::render_abc_file(path)?);
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
        assert!(
            !render.samples.is_empty(),
            "rendered sample buffer should be non-empty"
        );
    }

    #[test]
    fn jam_session_renders_distinct_sections() {
        use crate::ffi::JamSession;

        let engine = GuiAudioEngine::new();
        let demo = engine
            .demo_catalog()
            .into_iter()
            .find(|entry| entry.key == "dark_moroder" && entry.overview.is_some())
            .expect("dark_moroder demo should be available");

        let mut session =
            JamSession::open(&demo.path, 0xC0FFEE, 12.0).expect("jam session should open");
        let first = session.render_next().expect("first section should render");
        let second = session
            .render_next()
            .expect("second section should render");

        assert!(!first.samples.is_empty(), "first section must produce PCM");
        assert!(!second.samples.is_empty(), "second section must produce PCM");
        assert_eq!(first.iteration, 1, "iteration starts at 1");
        assert_eq!(second.iteration, 2, "iteration advances per section");
        assert_ne!(
            first.arrangement_offset, second.arrangement_offset,
            "scroll head must advance between sections"
        );
    }

    #[test]
    fn jam_worker_streams_sections_in_order() {
        // Mirrors the GUI's worker pattern: spawn a thread that owns the
        // session, push sections through a bounded channel, consume from
        // the main thread.
        use crate::ffi::{JamSection, JamSession};
        use std::sync::mpsc;

        let engine = GuiAudioEngine::new();
        let demo = engine
            .demo_catalog()
            .into_iter()
            .find(|entry| entry.key == "dark_moroder" && entry.overview.is_some())
            .expect("dark_moroder demo should be available");
        let session = JamSession::open(&demo.path, 0xC0FFEE, 12.0).unwrap();

        let (tx, rx) = mpsc::sync_channel::<Result<JamSection, String>>(1);
        let worker = std::thread::spawn(move || {
            let mut session = session;
            for _ in 0..3 {
                let s = session.render_next().expect("render");
                if tx.send(Ok(s)).is_err() {
                    break;
                }
            }
        });

        let mut iters = Vec::new();
        for _ in 0..3 {
            let section = rx.recv().expect("recv").expect("ok");
            assert!(!section.samples.is_empty());
            iters.push(section.iteration);
        }
        worker.join().expect("worker exits");
        assert_eq!(iters, vec![1, 2, 3], "sections arrive in order");
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
            assert!(
                !render.samples.is_empty(),
                "rendered sample buffer should be non-empty"
            );
        }
    }

    #[test]
    fn overview_contains_expected_tracks() {
        let engine = GuiAudioEngine::new();
        let demo = engine
            .demo_catalog()
            .into_iter()
            .find_map(|entry| entry.overview.map(|overview| (entry.key, overview)))
            .expect("at least one demo overview should load");

        assert!(
            !demo.1.tracks.is_empty(),
            "demo {} should expose track overviews",
            demo.0
        );
    }

    #[test]
    fn fx_overview_is_safe_for_all_tracks() {
        let engine = GuiAudioEngine::new();
        for entry in engine.demo_catalog() {
            let Some(overview) = entry.overview else {
                continue;
            };

            for track in &overview.tracks {
                assert!(
                    overview.fx_buses.get(track.fx_bus).is_some(),
                    "track {} in {} must resolve to a valid fx bus index {}",
                    track.name,
                    entry.key,
                    track.fx_bus
                );
            }
        }
    }

    #[test]
    fn invalid_abc_does_not_crash() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let invalid_path = std::env::temp_dir().join(format!(
            "memdeck-invalid-{unique}-{}.abc",
            std::process::id()
        ));
        fs::write(
            &invalid_path,
            [
                "X:1",
                "T:Invalid Overview",
                "M:4/4",
                "L:1/16",
                "Q:1/4=120",
                "K:C",
                "%%pattern A length=16",
                "%%arrangement Z",
                "V:lead",
                "| z16 |",
            ]
            .join("\n"),
        )
        .expect("should write invalid abc fixture");

        let result = std::panic::catch_unwind(|| crate::ffi::load_demo_overview(&invalid_path));
        let _ = fs::remove_file(&invalid_path);

        assert!(result.is_ok(), "invalid abc must not panic");
        assert!(
            result
                .expect("catch_unwind should return the parser result")
                .is_err(),
            "invalid abc should surface a recoverable error"
        );
    }
}
