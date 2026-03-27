// editor/src/commands/game/create_world_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::storage::editor_storage::create_new_world;
use crate::with_editor;
use engine_core::worlds::world::WorldId;

/// Undo-able command for creating a new world.
#[derive(Debug)]
pub struct CreateWorldCmd {
    world_id: Option<WorldId>,
}

impl CreateWorldCmd {
    pub fn new() -> Self {
        Self { world_id: None }
    }
}

impl EditorCommand for CreateWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;
            let world = create_new_world(game);
            self.world_id = Some(world.id);
            game.add_world(world);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;
            if let Some(id) = self.world_id.take() {
                game.delete_world(id);
                editor.save();
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }
}
