// editor/src/commands/asset/delete_unregistered_file_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use std::fs;
use std::path::PathBuf;

/// Undoable command that deletes an unregistered resource file.
#[derive(Debug)]
pub struct DeleteUnregisteredFileCmd {
    full_path: PathBuf,
    saved_bytes: Option<Vec<u8>>,
}

impl DeleteUnregisteredFileCmd {
    /// Creates a delete command for an unregistered file.
    pub fn new(full_path: PathBuf) -> Self {
        Self {
            full_path,
            saved_bytes: None,
        }
    }
}

impl EditorCommand for DeleteUnregisteredFileCmd {
    fn execute(&mut self) {
        let Ok(saved_bytes) = fs::read(&self.full_path) else {
            self.saved_bytes = None;
            return;
        };

        if fs::remove_file(&self.full_path).is_ok() {
            self.saved_bytes = Some(saved_bytes);
            with_editor(|editor| editor.save());
        } else {
            self.saved_bytes = None;
        }
    }

    fn undo(&mut self) {
        let Some(bytes) = self.saved_bytes.take() else {
            return;
        };

        if let Some(parent) = self.full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if fs::write(&self.full_path, bytes).is_ok() {
            with_editor(|editor| editor.save());
        }
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
