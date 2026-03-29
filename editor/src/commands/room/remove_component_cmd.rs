// editor/src/commands/room/remove_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for removing a component from an entity via the inspector.
#[derive(Debug)]
pub struct RemoveComponentCmd {
    entity: Entity,
    room_id: RoomId,
    type_name: &'static str,
    /// RON snapshot of the component captured before removal, used to restore on undo.
    snapshot: String,
}

impl RemoveComponentCmd {
    pub fn new(entity: Entity, room_id: RoomId, type_name: &'static str, snapshot: String) -> Self {
        Self {
            entity,
            room_id,
            type_name,
            snapshot,
        }
    }
}

impl EditorCommand for RemoveComponentCmd {
    fn execute(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                if (reg.has)(ctx.ecs, entity) {
                    let mut boxed = (reg.clone)(ctx.ecs, entity);
                    (reg.post_remove)(&mut *boxed, &entity, ctx);
                    (reg.remove)(ctx.ecs, entity);
                }
            }
        });
    }

    fn undo(&mut self) {
        let type_name = self.type_name;
        let snapshot = self.snapshot.clone();
        let entity = self.entity;
        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                let mut boxed = (reg.from_ron_component)(snapshot);
                (reg.post_create)(&mut *boxed, &entity, ctx);
                (reg.inserter)(ctx.ecs, entity, boxed);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
