// editor/src/commands/room/batch_move_entities_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for moving multiple entities at once.
#[derive(Debug)]
pub struct BatchMoveEntitiesCmd {
    /// Vector of (entity, from_position, to_position)
    pub moves: Vec<(Entity, Vec2, Vec2)>,
    pub room_id: RoomId,
}

impl BatchMoveEntitiesCmd {
    pub fn new(moves: Vec<(Entity, Vec2, Vec2)>, room_id: RoomId) -> Self {
        Self { moves, room_id }
    }
}

impl EditorCommand for BatchMoveEntitiesCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            for &(entity, _, to) in &self.moves {
                update_entity_position(ecs, entity, to);
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            for &(entity, from, _) in &self.moves {
                update_entity_position(ecs, entity, from);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
