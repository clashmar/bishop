use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for placing a linked prefab instance into a room.
#[derive(Debug)]
pub struct PlacePrefabInstanceCmd {
    mode: EditorMode,
    prefab_id: PrefabId,
    room_id: RoomId,
    position: Vec2,
    placed_root: Option<Entity>,
    snapshot: Option<GroupSnapshot>,
}

impl PlacePrefabInstanceCmd {
    pub fn new(prefab_id: PrefabId, room_id: RoomId, position: Vec2, mode: EditorMode) -> Self {
        Self {
            mode,
            prefab_id,
            room_id,
            position,
            placed_root: None,
            snapshot: None,
        }
    }
}

impl EditorCommand for PlacePrefabInstanceCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            if let Some(snapshot) = &self.snapshot {
                let mut game_ctx = editor.game.ctx_mut();
                restore_subtree(&mut game_ctx, snapshot);
                self.placed_root = snapshot.first().map(|entity| entity.entity);
            } else {
                let Some(prefab) = editor
                    .game
                    .prefab_manager
                    .prefabs
                    .get(&self.prefab_id)
                    .cloned()
                else {
                    return;
                };
                let root = {
                    let mut game_ctx = editor.game.ctx_mut();
                    instantiate_prefab(&mut game_ctx, &prefab, self.position, Some(self.room_id))
                };
                if root == Entity::null() {
                    return;
                }

                self.snapshot = Some(capture_subtree(&mut editor.game.ecs, root));
                self.placed_root = Some(root);
            }

            editor.room_editor.set_selected_entity(self.placed_root);
        });
    }

    fn undo(&mut self) {
        let Some(root) = self.placed_root else {
            return;
        };

        with_editor(|editor| {
            let mut game_ctx = editor.game.ctx_mut();
            Ecs::remove_entity(&mut game_ctx, root);
            editor.room_editor.set_selected_entity(None);
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
    use crate::editor_global::{reset_services, set_editor};
    use crate::storage::editor_storage::create_new_game;
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

    fn test_prefab(prefab_id: PrefabId) -> PrefabAsset {
        PrefabAsset {
            id: prefab_id,
            name: "Crate".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![
                    ComponentSnapshot {
                        type_name: comp_type_name::<Transform>().to_string(),
                        ron: ron::to_string(&Transform::default()).unwrap_or_default(),
                    },
                    ComponentSnapshot {
                        type_name: comp_type_name::<Name>().to_string(),
                        ron: ron::to_string(&Name("Crate".to_string())).unwrap_or_default(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn place_prefab_instance_command_restores_snapshot_on_redo() {
        let _lock = game_fs_test_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("place_prefab_instance_cmd");
        let mut game = create_new_game(test_game.name().to_string());
        let world_id = game
            .current_world_id
            .expect("test game should have a current world");
        let room_id = game.current_world().starting_room_id.unwrap_or_default();
        game.get_world_mut(world_id).current_room_id = Some(room_id);
        let prefab_id = PrefabId(1);
        game.prefab_manager
            .prefabs
            .insert(prefab_id, test_prefab(prefab_id));

        reset_services();
        set_editor(Editor {
            game,
            mode: EditorMode::Room(room_id),
            cur_world_id: Some(world_id),
            cur_room_id: Some(room_id),
            ..Default::default()
        });

        let mut cmd = PlacePrefabInstanceCmd::new(
            prefab_id,
            room_id,
            Vec2::new(32.0, 64.0),
            EditorMode::Room(room_id),
        );

        cmd.execute();

        let first_root = with_editor(|editor| {
            editor
                .room_editor
                .single_selected_entity()
                .unwrap_or(Entity::null())
        });
        assert_ne!(first_root, Entity::null());
        with_editor(|editor| {
            let root = editor.game.ecs.get::<PrefabInstanceRoot>(first_root);
            assert!(root.is_some_and(|root| root.prefab_id == prefab_id));
        });

        cmd.undo();
        with_editor(|editor| {
            assert!(!editor.game.ecs.has::<PrefabInstanceRoot>(first_root));
            assert_eq!(editor.room_editor.single_selected_entity(), None);
        });

        with_editor(|editor| {
            editor.game.prefab_manager.prefabs.insert(
                prefab_id,
                PrefabAsset {
                    name: "Updated".to_string(),
                    ..test_prefab(prefab_id)
                },
            );
        });

        cmd.execute();
        with_editor(|editor| {
            assert!(editor.game.ecs.has::<PrefabInstanceRoot>(first_root));
            let root = editor.game.ecs.get::<PrefabInstanceRoot>(first_root);
            assert!(root.is_some_and(|root| root.prefab_id == prefab_id));
            assert_eq!(
                editor.room_editor.single_selected_entity(),
                Some(first_root)
            );
        });
    }
}
