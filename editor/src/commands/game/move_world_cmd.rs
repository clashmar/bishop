// editor/src/commands/game/move_world_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use bishop::prelude::*;
use engine_core::game::Game;
use engine_core::worlds::world::WorldId;

/// Undo-able command for moving a world's position.
#[derive(Debug)]
pub struct MoveWorldCmd {
    world_id: WorldId,
    from: Vec2,
    to: Vec2,
}

impl MoveWorldCmd {
    pub fn new(world_id: WorldId, from: Vec2, to: Vec2) -> Self {
        Self { world_id, from, to }
    }

    /// Helper that sets the position of the world.
    fn set_position(game: &mut Game, world_id: WorldId, position: Vec2) {
        if let Some(world) = game.worlds.iter_mut().find(|w| w.id == world_id) {
            world.meta.position = position
        }
    }
}

impl EditorCommand for MoveWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;
            Self::set_position(game, self.world_id, self.to);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;
            Self::set_position(game, self.world_id, self.from);
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }
}
