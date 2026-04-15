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
    pub mode: EditorMode,
}

impl BatchMoveEntitiesCmd {
    pub fn new(moves: Vec<(Entity, Vec2, Vec2)>, mode: EditorMode) -> Self {
        Self { moves, mode }
    }
}

impl EditorCommand for BatchMoveEntitiesCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ecs = match editor.mode {
                EditorMode::Prefab(_) => &mut editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ecs,
                _ => &mut editor.game.ecs,
            };
            for &(entity, _, to) in &self.moves {
                update_entity_position(ecs, entity, to);
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let ecs = match editor.mode {
                EditorMode::Prefab(_) => &mut editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ecs,
                _ => &mut editor.game.ecs,
            };
            for &(entity, from, _) in &self.moves {
                update_entity_position(ecs, entity, from);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}
