use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::commands::scene::context::{with_scene_ctx, with_scene_ecs};
use crate::prefab::instance_sync::sync_prefab_overrides_for_entity;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for adding a component to an entity via the inspector.
#[derive(Debug)]
pub struct AddComponentCmd {
    entity: Entity,
    mode: EditorMode,
    type_name: &'static str,
}

impl AddComponentCmd {
    pub fn new(entity: Entity, mode: EditorMode, type_name: &'static str) -> Self {
        Self {
            entity,
            mode,
            type_name,
        }
    }
}

impl EditorCommand for AddComponentCmd {
    fn execute(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        let mode = self.mode;
        with_editor(|editor| {
            with_scene_ecs(editor, mode, |ecs| {
                if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                    (reg.factory)(ecs, entity);
                }
            });
            if matches!(mode, EditorMode::Room(_)) {
                sync_prefab_overrides_for_entity(
                    &mut editor.game.ecs,
                    &editor.game.prefab_manager,
                    entity,
                );
            }
        });
    }

    fn undo(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        let mode = self.mode;
        with_editor(|editor| {
            with_scene_ctx(editor, mode, |ctx| {
                if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                    if (reg.has)(ctx.ecs(), entity) {
                        let mut boxed = (reg.clone)(ctx.ecs(), entity);
                        (reg.post_remove)(&mut *boxed, &entity, ctx);
                        (reg.remove)(ctx.ecs(), entity);
                    }
                }
            });
            if matches!(mode, EditorMode::Room(_)) {
                sync_prefab_overrides_for_entity(
                    &mut editor.game.ecs,
                    &editor.game.prefab_manager,
                    entity,
                );
            }
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}
