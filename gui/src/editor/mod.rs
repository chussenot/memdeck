pub mod abc_load;
pub mod abc_save;
pub mod model;
pub mod state;
pub mod validation;

#[allow(unused_imports)]
pub use abc_load::{build_editable_song_from_raw, load_editable_song_from_path};
pub use abc_save::{save_editable_song_to_path, serialize_editable_song};
#[allow(unused_imports)]
pub use model::{
    EditableArrangement, EditableArrangementBlock, EditableFxBus, EditableInstrument, EditablePattern,
    EditableSong, EditableStep, EditableTrack,
};
pub use state::{EditorMode, EditorState};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn temp_abc_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("memdeck-editor-{name}-{}.abc", std::process::id()))
    }

    fn simple_abc() -> String {
        [
            "X:1",
            "T:Simple",
            "M:4/4",
            "L:1/16",
            "Q:1/4=120",
            "K:C",
            "% preserved comment",
            "%%swing 50",
            "%%instrument lead wave=square amp=64 duty=25 attack=0 decay=0 sustain=100 release=0 gate=90 fx=0",
            "%%effect 0 delay_steps=0 delay_feedback=0 delay_mix=0 drive=0 lowpass=0 sidechain=0 sidechain_release=180 mix=100",
            "%%pattern A length=16",
            "%%arrangement A",
            "V:lead instrument=lead",
            "V:lead",
            "| czzzczzzczzzczzz |",
        ]
        .join("\n")
    }

    #[test]
    fn load_simple_abc() {
        let path = temp_abc_path("load-simple");
        fs::write(&path, simple_abc()).expect("fixture write should succeed");

        let loaded = load_editable_song_from_path(&path).expect("simple abc should load");
        let _ = fs::remove_file(path);

        assert_eq!(loaded.title, "Simple");
        assert_eq!(loaded.tempo, 120);
        assert_eq!(loaded.swing, 50);
        assert!(!loaded.tracks.is_empty());
    }

    #[test]
    fn save_simple_abc() {
        let mut song = EditableSong::new_song();
        song.title = "Save Test".to_string();
        song.tracks[0].steps[0] = EditableStep::note(60);

        let path = temp_abc_path("save-simple");
        save_editable_song_to_path(&mut song, &path).expect("save should succeed");
        let saved = fs::read_to_string(&path).expect("saved file should read");
        let _ = fs::remove_file(path);

        assert!(saved.contains("T:Save Test"));
        assert!(saved.contains("%%arrangement A"));
    }

    #[test]
    fn load_save_roundtrip() {
        let src_path = temp_abc_path("roundtrip-src");
        fs::write(&src_path, simple_abc()).expect("fixture write should succeed");

        let mut song = load_editable_song_from_path(&src_path).expect("load should succeed");
        song.tempo = 132;
        song.mark_dirty();

        let out_path = temp_abc_path("roundtrip-out");
        save_editable_song_to_path(&mut song, &out_path).expect("save should succeed");

        let reloaded = load_editable_song_from_path(&out_path).expect("reloaded song should parse");
        let _ = fs::remove_file(src_path);
        let _ = fs::remove_file(out_path);

        assert_eq!(reloaded.tempo, 132);
        assert_eq!(reloaded.swing, song.swing);
        assert_eq!(reloaded.arrangement.blocks.len(), song.arrangement.blocks.len());
    }

    #[test]
    fn invalid_abc_returns_clean_error() {
        let path = temp_abc_path("invalid");
        fs::write(&path, "X:1\nT:Broken\n%%arrangement MISSING\nV:lead\n| z16 |")
            .expect("fixture write should succeed");

        let loaded = load_editable_song_from_path(&path);
        let _ = fs::remove_file(path);

        assert!(loaded.is_err());
        let message = loaded.err().unwrap_or_default();
        assert!(!message.trim().is_empty());
    }
}
