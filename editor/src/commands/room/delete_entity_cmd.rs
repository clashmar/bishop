// editor/src/commands/room/delete_entity_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
/// Undo-able command for deleting an entity and its children.
#[derive(Debug)]
pub struct DeleteEntityCmd {
    pub entity: Entity,
    pub mode: EditorMode,
    pub saved: Option<GroupSnapshot>,
    pub cleared_prefab_root: bool,
}

impl DeleteEntityCmd {
    pub fn new(entity: Entity, mode: EditorMode) -> Self {
        Self {
            entity,
            mode,
            saved: None,
            cleared_prefab_root: false,
        }
    }

    fn uses_prefab_context(&self) -> bool {
        matches!(self.mode, EditorMode::Prefab(_))
    }
}

impl EditorCommand for DeleteEntityCmd {
    fn execute(&mut self) {
        // Capture components before deleting
        with_editor(|editor| {
            let prefab_mode = self.uses_prefab_context();
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
            self.saved = Some(capture_subtree(ctx.ecs(), self.entity));
            Ecs::remove_entity(ctx, self.entity);
            if prefab_mode {
                let deleted_entities = self
                    .saved
                    .as_ref()
                    .map(|saved| {
                        saved
                            .iter()
                            .map(|snapshot| snapshot.entity)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let prefab_editor = editor.prefab_editor.as_mut().expect("Prefab editor missing");
                self.cleared_prefab_root = prefab_editor.root_entity == Some(self.entity);
                prefab_editor.clear_deleted_entities(&deleted_entities);
            } else {
                editor.room_editor.set_selected_entity(None);
            }
        });
    }

    fn undo(&mut self) {
        if let Some(saved) = self.saved.take() {
            with_editor(|editor| {
                let prefab_mode = self.uses_prefab_context();
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
                // Restore every entity and its components
                restore_subtree(ctx, &saved);
                if prefab_mode {
                    let prefab_editor = editor.prefab_editor.as_mut().expect("Prefab editor missing");
                    if self.cleared_prefab_root {
                        prefab_editor.restore_deleted_root(self.entity);
                    } else {
                        prefab_editor.set_selected_entity(Some(self.entity));
                    }
                } else {
                    editor.room_editor.set_selected_entity(Some(self.entity));
                }
            });
        }
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use crate::commands::editor_command_manager::EditorCommand;
    use crate::editor_global::{reset_services, set_editor, EDITOR_SERVICES};
    use crate::prefab::{PrefabEditor, PrefabStage};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use uuid::Uuid;

    struct EditorServicesGuard;

    impl EditorServicesGuard {
        fn install(editor: Editor) -> Self {
            reset_services();
            set_editor(editor);
            Self
        }
    }

    impl Drop for EditorServicesGuard {
        fn drop(&mut self) {
            EDITOR_SERVICES.with(|services| {
                *services.editor.borrow_mut() = None;
            });
            reset_services();
        }
    }

    #[test]
    fn prefab_delete_command_uses_stored_mode_instead_of_live_editor_mode() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("delete_cmd_prefab_mode");

        let mut editor = Editor {
            mode: EditorMode::Game,
            prefab_editor: Some(PrefabEditor::new(
                PrefabId(5),
                "Prefab".to_string(),
                None,
            )),
            prefab_stage: Some(PrefabStage::new(test_game.name())),
            ..Default::default()
        };

        let room_entity = editor
            .game
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Room Entity".to_string()))
            .finish();
        let prefab_root = editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Prefab Root".to_string()))
            .finish();

        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.root_entity = Some(prefab_root);
        prefab_editor.set_selected_entity(Some(prefab_root));

        let _guard = EditorServicesGuard::install(editor);

        let mut cmd = DeleteEntityCmd::new(prefab_root, EditorMode::Prefab(PrefabId(5)));
        cmd.execute();

        crate::with_editor(|editor| {
            assert!(editor.game.ecs.has::<Transform>(room_entity));
            assert!(!editor.prefab_stage.as_ref().unwrap().ecs.has::<Transform>(prefab_root));
            assert_eq!(editor.prefab_editor.as_ref().unwrap().root_entity, None);
        });

        cmd.undo();

        crate::with_editor(|editor| {
            assert!(editor.game.ecs.has::<Transform>(room_entity));
            assert!(editor.prefab_stage.as_ref().unwrap().ecs.has::<Transform>(prefab_root));
            assert_eq!(editor.prefab_editor.as_ref().unwrap().root_entity, Some(prefab_root));
        });
    }

    #[test]
    fn room_delete_command_uses_stored_mode_instead_of_live_editor_mode() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("delete_cmd_room_mode");

        let mut editor = Editor {
            mode: EditorMode::Prefab(PrefabId(8)),
            prefab_editor: Some(PrefabEditor::new(
                PrefabId(8),
                "Prefab".to_string(),
                None,
            )),
            prefab_stage: Some(PrefabStage::new(test_game.name())),
            ..Default::default()
        };
        editor.game.add_world(World {
            id: WorldId(Uuid::new_v4()),
            ..Default::default()
        });

        let room_entity = editor
            .game
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Room Entity".to_string()))
            .finish();
        let prefab_entity = editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Prefab Entity".to_string()))
            .finish();

        editor.room_editor.set_selected_entity(Some(room_entity));
        let _guard = EditorServicesGuard::install(editor);

        let mut cmd = DeleteEntityCmd::new(room_entity, EditorMode::Room(RoomId(3)));
        cmd.execute();

        crate::with_editor(|editor| {
            assert!(!editor.game.ecs.has::<Transform>(room_entity));
            assert!(editor.prefab_stage.as_ref().unwrap().ecs.has::<Transform>(prefab_entity));
            assert!(editor.room_editor.selected_entities.is_empty());
        });

        cmd.undo();

        crate::with_editor(|editor| {
            assert!(editor.game.ecs.has::<Transform>(room_entity));
            assert!(editor.prefab_stage.as_ref().unwrap().ecs.has::<Transform>(prefab_entity));
            assert_eq!(editor.room_editor.single_selected_entity(), Some(room_entity));
        });
    }
}
