// editor/src/commands/asset/create_directory_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use std::fs;
use std::path::PathBuf;

/// Undoable command that creates a resource directory.
#[derive(Debug)]
pub struct CreateDirectoryCmd {
    full_path: PathBuf,
    created: bool,
}

impl CreateDirectoryCmd {
    /// Creates a directory command for the given full path.
    pub fn new(full_path: PathBuf) -> Self {
        Self {
            full_path,
            created: false,
        }
    }
}

impl EditorCommand for CreateDirectoryCmd {
    fn execute(&mut self) {
        self.created = !self.full_path.exists();
        if fs::create_dir_all(&self.full_path).is_ok() {
            with_editor(|editor| editor.save());
        } else {
            self.created = false;
        }
    }

    fn undo(&mut self) {
        if self.created && fs::remove_dir_all(&self.full_path).is_ok() {
            self.created = false;
            with_editor(|editor| editor.save());
        }
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}
