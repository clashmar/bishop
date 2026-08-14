use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::commands::scene::component_dependency_cleanup::{
    present_dependency_closure,
    prune_recorded_dependency_components,
};
use crate::commands::scene::context::{with_scene_ctx, with_scene_ecs};
use crate::prefab::instance_sync::sync_prefab_overrides_for_entity;
use crate::with_editor;
use engine_core::ecs::*;
use std::collections::HashSet;

/// Undo-able command for adding a component to an entity via the inspector.
#[derive(Debug)]
pub struct AddComponentCmd {
    entity: Entity,
    mode: EditorMode,
    type_name: &'static str,
    created_dependency_type_names: Vec<&'static str>,
}

impl AddComponentCmd {
    pub fn new(entity: Entity, mode: EditorMode, type_name: &'static str) -> Self {
        Self {
            entity,
            mode,
            type_name,
            created_dependency_type_names: Vec::new(),
        }
    }
}

impl EditorCommand for AddComponentCmd {
    fn execute(&mut self) {
        let type_name = self.type_name;
        let entity = self.entity;
        let mode = self.mode;
        let created_dependency_type_names = &mut self.created_dependency_type_names;
        with_editor(|editor| {
            with_scene_ecs(editor, mode, |ecs| {
                created_dependency_type_names.clear();

                // FLAG: If we start adding more special cases
                // consider defining this behaviour on the component
                if type_name == CurrentRoom::TYPE_NAME {
                    if let EditorMode::Room(room_id) = mode {
                        ecs.set_current_room(entity, room_id);
                    }
                    return;
                }

                if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                    let before_dependencies: HashSet<_> =
                        present_dependency_closure(ecs, entity, type_name)
                            .into_iter()
                            .collect();
                    (reg.factory)(ecs, entity);
                    *created_dependency_type_names = present_dependency_closure(ecs, entity, type_name)
                        .into_iter()
                        .filter(|dependency| !before_dependencies.contains(dependency))
                        .collect();
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
        let created_dependency_type_names = std::mem::take(&mut self.created_dependency_type_names);
        with_editor(|editor| {
            with_scene_ctx(editor, mode, |ctx| {
                // FLAG: If we start adding more special cases
                // consider defining this behaviour on the component
                if type_name == CurrentRoom::TYPE_NAME {
                    Ecs::remove_component::<CurrentRoom>(ctx, entity);
                    return;
                }

                Ecs::remove_component_by_type_name(ctx, entity, type_name);
                prune_recorded_dependency_components(
                    ctx,
                    entity,
                    &created_dependency_type_names,
                );
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
    use engine_core::worlds::*;

    #[test]
    fn undoing_component_add_removes_auto_created_dependency_graph() {
        reset_services();
        set_editor(make_editor_with_world());

        let entity = with_editor(|editor| editor.game.ecs.create_entity().finish());

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(1)),
            PhysicsBody::TYPE_NAME,
        );
        cmd.execute();
        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<PhysicsBody>(entity));
            assert_physics_body_dependency_closure(&editor.game.ecs, entity, false, false);
        });
    }

    #[test]
    fn adding_layer_door_auto_adds_interactable_and_undo_removes_both() {
        reset_services();
        set_editor(make_editor_with_world());

        let entity = with_editor(|editor| editor.game.ecs.create_entity().finish());

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(1)),
            LayerDoor::TYPE_NAME,
        );
        cmd.execute();

        with_editor(|editor| {
            assert!(editor.game.ecs.has::<LayerDoor>(entity));
            assert!(editor.game.ecs.has::<Interactable>(entity));
        });

        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<LayerDoor>(entity));
            assert!(!editor.game.ecs.has::<Interactable>(entity));
        });
    }

    #[test]
    fn undoing_component_add_keeps_pre_existing_dependency_graph() {
        reset_services();
        set_editor(make_editor_with_world());

        let entity = with_editor(|editor| {
            editor
                .game
                .ecs
                .create_entity()
                .with(MotionBody)
                .finish()
        });

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(1)),
            PhysicsBody::TYPE_NAME,
        );
        cmd.execute();
        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<PhysicsBody>(entity));
            assert_physics_body_dependency_closure(&editor.game.ecs, entity, true, true);
        });
    }

    #[test]
    fn undoing_motion_body_add_prunes_orphaned_subpixel() {
        reset_services();
        set_editor(make_editor_with_world());

        let entity = with_editor(|editor| editor.game.ecs.create_entity().finish());

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(1)),
            MotionBody::TYPE_NAME,
        );
        cmd.execute();
        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<MotionBody>(entity));
            assert!(!editor.game.ecs.has::<SubPixel>(entity));
        });
    }

    #[test]
    fn room_component_add_assigns_membership_and_undo_clears_it() {
        reset_services();
        set_editor(make_editor_with_world());

        let entity = with_editor(|editor| editor.game.ecs.create_entity().finish());

        let mut cmd = AddComponentCmd::new(
            entity,
            EditorMode::Room(RoomId(7)),
            CurrentRoom::TYPE_NAME,
        );
        cmd.execute();

        with_editor(|editor| {
            assert_eq!(
                editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.room_id),
                Some(RoomId(7))
            );
            assert!(editor.game.ecs.entities_in_room(RoomId(7)).contains(&entity));
        });

        cmd.undo();

        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<CurrentRoom>(entity));
            assert!(!editor.game.ecs.entities_in_room(RoomId(7)).contains(&entity));
        });
    }

    fn make_editor_with_world() -> Editor {
        let mut editor = Editor::default();
        editor.game.add_world(Default::default());
        editor
    }

    fn assert_physics_body_dependency_closure(
        ecs: &Ecs,
        entity: Entity,
        expect_motion_body: bool,
        expect_sub_pixel: bool,
    ) {
        assert!(!ecs.has::<Active>(entity));
        assert!(!ecs.has::<Collider>(entity));
        assert!(!ecs.has::<Grounded>(entity));
        assert_eq!(ecs.has::<MotionBody>(entity), expect_motion_body);
        assert_eq!(ecs.has::<SubPixel>(entity), expect_sub_pixel);
        assert!(!ecs.has::<Transform>(entity));
        assert!(!ecs.has::<Velocity>(entity));
    }
}
