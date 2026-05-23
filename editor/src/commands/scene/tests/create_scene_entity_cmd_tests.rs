use crate::app::EditorMode;
use crate::commands::scene::CreateSceneEntityCmd;
use crate::editor_global::{
    apply_pending_commands, push_command, request_redo, request_undo, with_editor,
};
use crate::test_utils::{EditorServicesGuard, TestGameFolder, game_fs_test_lock, make_room_editor};
use engine_core::prelude::*;

fn find_room_entity(ecs: &Ecs, room_id: RoomId, parent: Option<Entity>) -> Option<Entity> {
    ecs.get_store::<Name>()
        .data
        .iter()
        .find_map(|(&entity, name)| {
            let in_room = ecs
                .get::<CurrentRoom>(entity)
                .is_some_and(|current_room| current_room.0 == room_id);
            let named_entity = name.0 == CreateSceneEntityCmd::ROOM_ENTITY_NAME;
            let matching_parent =
                ecs.get::<Parent>(entity).map(|parent_comp| parent_comp.0) == parent;
            (in_room && named_entity && matching_parent).then_some(entity)
        })
}

fn find_global_entity(ecs: &Ecs) -> Option<Entity> {
    ecs.get_store::<Global>()
        .data
        .iter()
        .find_map(|(&entity, _)| {
            ecs.get::<Name>(entity)
                .is_some_and(|name| name.0 == CreateSceneEntityCmd::GLOBAL_ENTITY_NAME)
                .then_some(entity)
        })
}

fn find_player_proxy(ecs: &Ecs, room_id: RoomId) -> Option<Entity> {
    ecs.get_store::<PlayerProxy>()
        .data
        .iter()
        .find_map(|(&entity, _)| {
            let in_room = ecs
                .get::<CurrentRoom>(entity)
                .is_some_and(|current_room| current_room.0 == room_id);
            let named_proxy = ecs
                .get::<Name>(entity)
                .is_some_and(|name| name.0 == CreateSceneEntityCmd::PLAYER_PROXY_NAME);
            (in_room && named_proxy).then_some(entity)
        })
}

#[test]
fn room_entity_create_command_assigns_parent_and_supports_undo_redo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("create_scene_room_entity_cmd");
    let (mut editor, room_id) = make_room_editor(&test_game);
    let parent = editor
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Parent".to_string()))
        .with_current_room(room_id)
        .finish();
    let _guard = EditorServicesGuard::install(editor);

    push_command(Box::new(CreateSceneEntityCmd::new_room_entity(
        room_id,
        Vec2::new(32.0, 48.0),
        Some(parent),
    )));
    apply_pending_commands();

    let created = with_editor(|editor| {
        let ecs = &editor.game.ecs;
        let entity = find_room_entity(ecs, room_id, Some(parent)).expect("entity should exist");
        assert_eq!(
            ecs.get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(32.0, 48.0))
        );
        assert_eq!(editor.room_editor.single_selected_entity(), Some(entity));
        entity
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            editor.game.ecs.get::<Name>(created).is_none(),
            "undo should remove the created entity"
        );
        assert_eq!(editor.room_editor.single_selected_entity(), None);
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        let ecs = &editor.game.ecs;
        let recreated =
            find_room_entity(ecs, room_id, Some(parent)).expect("redo should recreate entity");
        assert_eq!(
            ecs.get::<Transform>(recreated)
                .map(|transform| transform.position),
            Some(Vec2::new(32.0, 48.0))
        );
        assert_eq!(editor.room_editor.single_selected_entity(), Some(recreated));
    });
}

#[test]
fn global_entity_create_command_supports_undo_redo_from_room_mode() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("create_scene_global_entity_cmd_room_mode");
    let (editor, room_id) = make_room_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    push_command(Box::new(CreateSceneEntityCmd::new_global_entity(room_id)));
    apply_pending_commands();

    let created = with_editor(|editor| {
        assert!(matches!(editor.mode, EditorMode::Room(_)));
        find_global_entity(&editor.game.ecs).expect("global entity should exist")
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            editor.game.ecs.get::<Name>(created).is_none(),
            "undo should remove the global entity even from room mode"
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            find_global_entity(&editor.game.ecs).is_some(),
            "redo should recreate the global entity from room mode"
        );
        assert!(matches!(editor.mode, EditorMode::Room(_)));
    });
}

#[test]
fn global_entity_create_command_supports_undo_redo_after_switching_rooms() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("create_scene_global_entity_cmd_room_switch");
    let (mut editor, room_id) = make_room_editor(&test_game);
    let world_id = editor.cur_world_id.expect("room editor should have current world");
    let second_room_id = editor.game.id_allocator.allocate_room_id();
    let grid_size = editor.game.current_world().grid_size;
    let second_room = Room::new(&mut editor.game.ecs, second_room_id, grid_size);
    editor
        .game
        .get_world_mut(world_id)
        .expect("current world should exist")
        .add_room(second_room);
    let _guard = EditorServicesGuard::install(editor);

    push_command(Box::new(CreateSceneEntityCmd::new_global_entity(room_id)));
    apply_pending_commands();

    let created = with_editor(|editor| {
        find_global_entity(&editor.game.ecs).expect("global entity should exist")
    });

    with_editor(|editor| {
        editor.mode = EditorMode::Room(second_room_id);
        editor.cur_room_id = Some(second_room_id);
        editor
            .game
            .get_world_mut(world_id)
            .expect("current world should exist")
            .current_room_id = Some(second_room_id);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            editor.game.ecs.get::<Name>(created).is_none(),
            "undo in another room should still remove the global entity"
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            find_global_entity(&editor.game.ecs).is_some(),
            "redo in another room should recreate the global entity"
        );
        assert!(matches!(editor.mode, EditorMode::Room(_)));
    });
}

#[test]
fn player_proxy_create_command_supports_undo_redo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("create_scene_player_proxy_cmd");
    let (mut editor, room_id) = make_room_editor(&test_game);
    if let Some(existing_proxy) = editor.game.ecs.get_player_proxy(room_id) {
        let ctx = &mut editor.game.ctx_mut();
        Ecs::remove_entity(ctx, existing_proxy);
    }
    let _guard = EditorServicesGuard::install(editor);

    push_command(Box::new(CreateSceneEntityCmd::new_player_proxy(
        room_id,
        Vec2::new(64.0, 96.0),
    )));
    apply_pending_commands();

    let created = with_editor(|editor| {
        let ecs = &editor.game.ecs;
        let entity = find_player_proxy(ecs, room_id).expect("player proxy should exist");
        assert_eq!(
            ecs.get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(64.0, 96.0))
        );
        entity
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(
            editor.game.ecs.get::<PlayerProxy>(created).is_none(),
            "undo should remove the player proxy"
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        let ecs = &editor.game.ecs;
        let recreated = find_player_proxy(ecs, room_id).expect("redo should recreate player proxy");
        assert_eq!(
            ecs.get::<Transform>(recreated)
                .map(|transform| transform.position),
            Some(Vec2::new(64.0, 96.0))
        );
    });
}
