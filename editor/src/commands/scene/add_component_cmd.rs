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
                // FLAG: If we start adding more special cases
                // consider defining this behaviour on the component
                if type_name == CurrentRoom::TYPE_NAME {
                    if let EditorMode::Room(room_id) = mode {
                        ecs.set_current_room(entity, room_id);
                    }
                    return;
                }

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
    fn room_component_add_assigns_membership_and_undo_clears_it() {
        reset_services();

        let mut editor = Editor::default();
        editor.game.add_world(Default::default());
        set_editor(editor);

        let entity = with_editor(|editor| editor.game.ecs.create_entity().finish());

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(7)),
            CurrentRoom::TYPE_NAME,
        );
        cmd.execute();

        with_editor(|editor| {
            assert_eq!(editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.0), Some(RoomId(7)));
            assert!(editor.game.ecs.entities_in_room(RoomId(7)).contains(&entity));
        });

        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<CurrentRoom>(entity));
            assert!(!editor.game.ecs.entities_in_room(RoomId(7)).contains(&entity));
        });
    }
}
