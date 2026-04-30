// editor/src/commands/game/delete_world_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;
use std::collections::HashSet;

/// Undo-able command for deleting a world.
#[derive(Debug)]
pub struct DeleteWorldCmd {
    world_id: WorldId,
    deleted_world_index: Option<usize>,
    deleted_world: Option<World>,
    prev_current_world: Option<WorldId>,
    saved_entities: Option<GroupSnapshot>,
}

impl DeleteWorldCmd {
    pub fn new(game: &mut Game, world_id: WorldId) -> Self {
        Self {
            world_id,
            deleted_world_index: None,
            deleted_world: None,
            prev_current_world: game.current_world_id,
            saved_entities: None,
        }
    }
}

impl EditorCommand for DeleteWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let game = &mut editor.game;

            if let Some((world_index, world)) = game
                .worlds
                .iter()
                .enumerate()
                .find(|(_, w)| w.id == self.world_id)
                .map(|(index, world)| (index, world.clone()))
            {
                self.deleted_world_index = Some(world_index);
                let room_ids: HashSet<RoomId> = world.rooms.iter().map(|room| room.id).collect();
                let entity_ids: HashSet<Entity> = {
                    let store = game.ecs.get_store::<CurrentRoom>();
                    store
                        .data
                        .iter()
                        .filter(|(_, CurrentRoom(room))| room_ids.contains(room))
                        .map(|(&entity, _)| entity)
                        .collect()
                };

                let root_entities = get_root_entities_in_set(&game.ecs, &entity_ids);

                let mut saved_entities = Vec::new();
                for entity in root_entities {
                    saved_entities.extend(capture_subtree(&mut game.ecs, entity));
                }

                self.deleted_world = Some(world);
                self.saved_entities = Some(saved_entities);
            }

            game.delete_world(self.world_id);
            editor.save();
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            if let Some(world) = self.deleted_world.take() {
                let index = self
                    .deleted_world_index
                    .unwrap_or(editor.game.worlds.len())
                    .min(editor.game.worlds.len());
                editor.game.worlds.insert(index, world);
            }

            editor.game.current_world_id = self.prev_current_world;

            if let Some(saved) = self.saved_entities.take() {
                let ctx = &mut editor.game.ctx_mut();
                restore_subtree(ctx, &saved);
            }

            editor.save();
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Game
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use crate::editor_global::with_editor;
    use crate::storage::editor_storage::{create_new_game, create_new_world};
    use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};

    #[test]
    fn delete_world_cmd_deletes_and_restores_world_entities() {
        let _lock = game_fs_test_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("delete_world_cmd");
        set_game_name(test_game.name());

        let mut game = create_new_game(test_game.name().to_string());
        assert_eq!(game.worlds.len(), 1);
        let world_id = game.worlds[0].id;
        assert!(!game.worlds[0].rooms.is_empty());
        let room_id = game.worlds[0].rooms[0].id;

        let extra_world = create_new_world(&mut game);
        let extra_world_id = extra_world.id;
        assert!(!extra_world.rooms.is_empty());
        game.add_world(extra_world);

        let second_extra_world = create_new_world(&mut game);
        let second_extra_world_id = second_extra_world.id;
        assert!(!second_extra_world.rooms.is_empty());
        game.add_world(second_extra_world);

        game.select_world(world_id);

        let entity_name = format!("{}-entity", test_game.name());
        let entity = game
            .ecs
            .create_entity()
            .with(CurrentRoom(room_id))
            .with(Name(entity_name.clone()))
            .finish();

        let mut cmd = DeleteWorldCmd::new(&mut game, world_id);

        let editor = Editor {
            game,
            mode: EditorMode::Game,
            ..Default::default()
        };
        let _guard = EditorServicesGuard::install(editor);

        cmd.execute();

        with_editor(|editor| {
            assert_eq!(editor.game.worlds.len(), 2);
            assert_eq!(editor.game.current_world_id, Some(extra_world_id));
            assert!(!editor.game.ecs.has::<CurrentRoom>(entity));
            assert!(!editor.game.ecs.has::<Name>(entity));
        });

        cmd.undo();

        with_editor(|editor| {
            assert_eq!(editor.game.worlds.len(), 3);
            assert_eq!(editor.game.worlds[0].id, world_id);
            assert_eq!(editor.game.worlds[1].id, extra_world_id);
            assert_eq!(editor.game.worlds[2].id, second_extra_world_id);
            assert_eq!(editor.game.current_world_id, Some(world_id));
            assert!(editor.game.ecs.has::<CurrentRoom>(entity));
            assert!(editor.game.ecs.has::<Name>(entity));
            assert_eq!(
                editor.game.ecs.get::<Name>(entity).map(|name| &name.0),
                Some(&entity_name)
            );
        });
    }

    #[test]
    fn delete_world_cmd_restores_entity_hierarchy() {
        let _lock = game_fs_test_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("delete_world_cmd_hierarchy");
        set_game_name(test_game.name());

        let mut game = create_new_game(test_game.name().to_string());
        assert_eq!(game.worlds.len(), 1);
        let world_id = game.worlds[0].id;
        assert!(!game.worlds[0].rooms.is_empty());
        let room_id = game.worlds[0].rooms[0].id;

        let extra_world = create_new_world(&mut game);
        let extra_world_id = extra_world.id;
        assert!(!extra_world.rooms.is_empty());
        let extra_room_id = extra_world.rooms[0].id;
        game.add_world(extra_world);
        game.select_world(world_id);

        let root_name = format!("{}-root", test_game.name());
        let child_name = format!("{}-child", test_game.name());

        let root = game
            .ecs
            .create_entity()
            .with(CurrentRoom(room_id))
            .with(Name(root_name.clone()))
            .finish();
        let child = game
            .ecs
            .create_entity()
            .with(CurrentRoom(extra_room_id))
            .with(Name(child_name.clone()))
            .finish();
        set_parent(&mut game.ecs, child, root);

        let mut cmd = DeleteWorldCmd::new(&mut game, world_id);

        let editor = Editor {
            game,
            mode: EditorMode::Game,
            ..Default::default()
        };
        let _guard = EditorServicesGuard::install(editor);

        cmd.execute();

        with_editor(|editor| {
            assert_eq!(editor.game.worlds.len(), 1);
            assert_eq!(editor.game.worlds[0].id, extra_world_id);
            assert!(!editor.game.ecs.has::<CurrentRoom>(root));
            assert!(!editor.game.ecs.has::<CurrentRoom>(child));
        });

        cmd.undo();

        with_editor(|editor| {
            assert_eq!(editor.game.worlds.len(), 2);
            assert_eq!(editor.game.worlds[0].id, world_id);
            assert_eq!(editor.game.worlds[1].id, extra_world_id);
            assert!(editor.game.ecs.has::<Name>(root));
            assert!(editor.game.ecs.has::<Name>(child));
            assert_eq!(
                editor.game.ecs.get::<Parent>(child).map(|parent| parent.0),
                Some(root)
            );
            assert!(editor
                .game
                .ecs
                .get::<Children>(root)
                .is_some_and(|children| children.contains(child)));
            assert_eq!(
                editor.game.ecs.get::<Name>(root).map(|name| &name.0),
                Some(&root_name)
            );
            assert_eq!(
                editor.game.ecs.get::<Name>(child).map(|name| &name.0),
                Some(&child_name)
            );
        });
    }
}
