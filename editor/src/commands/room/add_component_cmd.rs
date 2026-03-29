// editor/src/commands/room/add_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for adding a component to an entity via the inspector.
#[derive(Debug)]
pub struct AddComponentCmd {
    entity: Entity,
    room_id: RoomId,
    type_name: &'static str,
}

impl AddComponentCmd {
    pub fn new(entity: Entity, room_id: RoomId, type_name: &'static str) -> Self {
        Self {
            entity,
            room_id,
            type_name,
        }
    }
}

impl EditorCommand for AddComponentCmd {
    fn execute(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        with_editor(|editor| {
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                (reg.factory)(&mut editor.game.ecs, entity);
            }
        });
    }

    fn undo(&mut self) {
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

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
