use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Browser,
    Edit,
    Preview,
}

impl EditorMode {
    pub fn label(self) -> &'static str {
        match self {
            EditorMode::Browser => "BROWSER",
            EditorMode::Edit => "EDIT",
            EditorMode::Preview => "PREVIEW",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EditorState {
    pub mode: EditorMode,
    pub selected_pattern: Option<usize>,
    pub selected_track: usize,
    pub selected_step: Option<usize>,
    pub selected_arrangement_block: Option<usize>,
    pub dirty: bool,
    pub last_saved_path: Option<PathBuf>,
    pub last_error: Option<String>,
}

impl EditorState {
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
    }
}
