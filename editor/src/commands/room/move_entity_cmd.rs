// editor/src/commands/room/move_entity_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for moving an entity.
#[derive(Debug)]
pub struct MoveEntityCmd {
    entity: Entity,
    mode: EditorMode,
    from: Vec2,
    to: Vec2,
    executed: bool,
}

impl MoveEntityCmd {
    pub fn new(entity: Entity, mode: EditorMode, from: Vec2, to: Vec2) -> Self {
        Self {
            entity,
            mode,
            from,
            to,
            executed: false,
        }
    }
}

impl EditorCommand for MoveEntityCmd {
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
            update_entity_position(ecs, self.entity, self.to);
        });
        self.executed = true;
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
            update_entity_position(ecs, self.entity, self.from);
        });
        self.executed = false;
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}
