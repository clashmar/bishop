// editor/src/commands/room/remove_component_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for removing a component from an entity via the inspector.
#[derive(Debug)]
pub struct RemoveComponentCmd {
    entity: Entity,
    mode: EditorMode,
    type_name: &'static str,
    /// RON snapshot of the component captured before removal, used to restore on undo.
    snapshot: String,
}

impl RemoveComponentCmd {
    pub fn new(entity: Entity, mode: EditorMode, type_name: &'static str, snapshot: String) -> Self {
        Self {
            entity,
            mode,
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
            let prefab_mode = matches!(editor.mode, EditorMode::Prefab(_));
            let mut prefab_ctx;
            let mut room_game_ctx;
            let mut room_services_ctx;
            let ctx: &mut dyn EngineCtxMut = if prefab_mode {
                prefab_ctx = editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ctx_mut();
                &mut prefab_ctx
            } else {
                room_game_ctx = editor.game.ctx_mut();
                room_services_ctx = room_game_ctx.services_ctx_mut();
                &mut room_services_ctx
            };
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                if (reg.has)(ctx.ecs(), entity) {
                    let mut boxed = (reg.clone)(ctx.ecs(), entity);
                    (reg.post_remove)(&mut *boxed, &entity, ctx);
                    (reg.remove)(ctx.ecs(), entity);
                }
            }

            if type_name == Animation::TYPE_NAME {
                Ecs::remove_component::<CurrentFrame>(ctx, entity);
            }
        });
    }

    fn undo(&mut self) {
        let type_name = self.type_name;
        let snapshot = self.snapshot.clone();
        let entity = self.entity;
        with_editor(|editor| {
            let prefab_mode = matches!(editor.mode, EditorMode::Prefab(_));
            let mut prefab_ctx;
            let mut room_game_ctx;
            let mut room_services_ctx;
            let ctx: &mut dyn EngineCtxMut = if prefab_mode {
                prefab_ctx = editor
                    .prefab_stage
                    .as_mut()
                    .expect("Prefab stage missing")
                    .ctx_mut();
                &mut prefab_ctx
            } else {
                room_game_ctx = editor.game.ctx_mut();
                room_services_ctx = room_game_ctx.services_ctx_mut();
                &mut room_services_ctx
            };
            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                let mut boxed = (reg.from_ron_component)(snapshot);
                (reg.post_create)(&mut *boxed, &entity, ctx);
                (reg.inserter)(ctx.ecs(), entity, boxed);
            }
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use crate::editor_global::{reset_services, set_editor, with_editor};

    #[test]
    fn removing_animation_component_also_removes_current_frame() {
        reset_services();

        let mut editor = Editor::default();
        editor.game.add_world(Default::default());
        set_editor(editor);

        let entity = with_editor(|editor| {
            let entity = editor.game.ecs.create_entity().finish();
            editor
                .game
                .ecs
                .add_component_to_entity(entity, Animation::default());
            editor.game.ecs.add_component_to_entity(
                entity,
                CurrentFrame {
                    sprite_id: SpriteId(7),
                    ..Default::default()
                },
            );
            entity
        });

        let snapshot = with_editor(|editor| {
            let reg = COMPONENTS
                .iter()
                .find(|r| r.type_name == Animation::TYPE_NAME)
                .expect("Animation component must be registered");
            let boxed = (reg.clone)(&mut editor.game.ecs, entity);
            (reg.to_ron_component)(boxed.as_ref())
        });

        let mut cmd = RemoveComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(1)),
            Animation::TYPE_NAME,
            snapshot,
        );
        cmd.execute();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<Animation>(entity));
            assert!(!editor.game.ecs.has::<CurrentFrame>(entity));
        });
    }
}
