// editor/src/commands/game/rename_game_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::storage::editor_storage::rename_game;
use crate::with_editor;

/// Undo-able command for renaming a game.
#[derive(Debug)]
pub struct RenameGameCmd {
    pub new_name: String,
    pub old_name: String,
}

impl RenameGameCmd {
    pub fn new(new_name: String, old_name: String) -> Self {
        Self { new_name, old_name }
    }
}

impl EditorCommand for RenameGameCmd {
    fn execute(&mut self) {
        with_editor(
            |editor| match rename_game(&mut editor.game, &self.new_name) {
                Ok(()) => {
                    editor.save();
                }
                Err(err) => {
                    push_toast(format!("Failed to rename game: {err}"), 3.0);
                }
            },
        );
    }

    fn undo(&mut self) {
        with_editor(
            |editor| match rename_game(&mut editor.game, &self.old_name) {
                Ok(()) => {
                    editor.save();
                }
                Err(err) => {
                    push_toast(format!("Failed to rename game: {err}"), 3.0);
                }
            },
        );
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }
}
