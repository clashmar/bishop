// editor/src/commands/game/delete_world_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for deleting a world.
#[derive(Debug)]
pub struct DeleteWorldCmd {
    world_id: WorldId,
    deleted_world: Option<World>,
    prev_current_world: WorldId,
}

impl DeleteWorldCmd {
    pub fn new(game: &mut Game, world_id: WorldId) -> Self {
        Self {
            world_id,
            deleted_world: None,
            prev_current_world: game.current_world_id,
        }
    }
}

impl EditorCommand for DeleteWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;

            // Capture world before deleting
            if let Some(pos) = game.worlds.iter().position(|w| w.id == self.world_id) {
                self.deleted_world = Some(game.worlds.swap_remove(pos));
            }

            // Clean up
            game.delete_world(self.world_id);
            editor.save();
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            // Push the world back into the game
            if let Some(world) = self.deleted_world.take() {
                editor.game.worlds.push(world);
            }

            // Restore previous active world
            editor.game.current_world_id = self.prev_current_world;
            editor.save();
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }
}
