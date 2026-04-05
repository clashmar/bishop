// editor/src/commands/room/add_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
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
        with_editor(|editor| {
            let ecs = match editor.mode {
                EditorMode::Prefab(_) => &mut editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ecs,
                _ => &mut editor.game.ecs,
            };
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                (reg.factory)(ecs, entity);
            }
        });
    }

    fn undo(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        with_editor(|editor| {
            let prefab_mode = matches!(editor.mode, EditorMode::Prefab(_));
            let mut game_ctx = (!prefab_mode).then(|| editor.game.ctx_mut());
            let mut prefab_ctx = prefab_mode.then(|| {
                editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ctx_mut()
            });
            let ctx = if let Some(ctx) = prefab_ctx.as_mut() {
                ctx
            } else if let Some(ctx) = game_ctx.as_mut() {
                &mut ctx.services_ctx_mut()
            } else {
                return;
            };
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
        self.mode
    }
}
