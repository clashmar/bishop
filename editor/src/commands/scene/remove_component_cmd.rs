use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::commands::scene::context::with_scene_ctx;
use crate::prefab::instance_sync::sync_prefab_overrides_for_entity;
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
    pub fn new(
        entity: Entity,
        mode: EditorMode,
        type_name: &'static str,
        snapshot: String,
    ) -> Self {
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
        let mode = self.mode;
        with_editor(|editor| {
            with_scene_ctx(editor, mode, |ctx| {
                // FLAG: If we start adding more special cases
                // consider defining this behaviour on the component
                if type_name == CurrentRoom::TYPE_NAME {
                    Ecs::remove_component::<CurrentRoom>(ctx, entity);
                    return;
                }

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
        let snapshot = self.snapshot.clone();
        let entity = self.entity;
        let mode = self.mode;
        with_editor(|editor| {
            with_scene_ctx(editor, mode, |ctx| {
                // FLAG: If we start adding more special cases
                // consider defining this behaviour on the component
                if type_name == CurrentRoom::TYPE_NAME {
                    let CurrentRoom(room_id) = ron::from_str::<CurrentRoom>(&snapshot)
                        .expect("CurrentRoom snapshot should deserialize");
                    ctx.ecs().set_current_room(entity, room_id);
                    return;
                }

                if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                    let mut boxed = (reg.from_ron_component)(snapshot);
                    (reg.post_create)(&mut *boxed, &entity, ctx);
                    (reg.inserter)(ctx.ecs(), entity, boxed);
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

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        self.mode == current_mode
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

    #[test]
    fn room_component_remove_clears_membership_and_undo_restores_it() {
        reset_services();

        let mut editor = Editor::default();
        editor.game.add_world(Default::default());
        set_editor(editor);

        let entity = with_editor(|editor| {
            editor
                .game
                .ecs
                .create_entity()
                .with_current_room(RoomId(3))
                .finish()
        });

        let snapshot = ron::to_string(&CurrentRoom(RoomId(3))).expect("CurrentRoom should serialize");
        let mut cmd = RemoveComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(3)),
            CurrentRoom::TYPE_NAME,
            snapshot,
        );

        cmd.execute();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<CurrentRoom>(entity));
            assert!(!editor.game.ecs.entities_in_room(RoomId(3)).contains(&entity));
        });

        cmd.undo();

        with_editor(|editor| {
            assert_eq!(editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.0), Some(RoomId(3)));
            assert!(editor.game.ecs.entities_in_room(RoomId(3)).contains(&entity));
        });
    }
}
