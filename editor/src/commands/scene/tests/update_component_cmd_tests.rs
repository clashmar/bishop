use super::*;
use crate::editor_global::{reset_services, set_editor};

#[test]
fn room_component_updates_move_membership_between_rooms() {
    reset_services();

    let mut editor = Editor::default();
    editor.game.add_world(Default::default());
    let entity = editor
        .game
        .ecs
        .create_entity()
        .with_current_room(RoomId(1))
        .finish();
    set_editor(editor);

    let old_ron = ron::to_string(&CurrentRoom(RoomId(1))).expect("CurrentRoom should serialize");
    let new_ron = ron::to_string(&CurrentRoom(RoomId(2))).expect("CurrentRoom should serialize");

    let mut cmd = UpdateComponentCmd::new(
        entity,
        EditorMode::Room(RoomId(1)),
        CurrentRoom::TYPE_NAME,
        old_ron,
        new_ron,
        Default::default(),
        Default::default(),
    );
    cmd.execute();

    with_editor(|editor| {
        assert_eq!(editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.0), Some(RoomId(2)));
        assert!(!editor.game.ecs.entities_in_room(RoomId(1)).contains(&entity));
        assert!(editor.game.ecs.entities_in_room(RoomId(2)).contains(&entity));
    });

    cmd.undo();

    with_editor(|editor| {
        assert_eq!(editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.0), Some(RoomId(1)));
        assert!(editor.game.ecs.entities_in_room(RoomId(1)).contains(&entity));
        assert!(!editor.game.ecs.entities_in_room(RoomId(2)).contains(&entity));
    });
}

#[test]
fn room_component_edits_write_prefab_overrides_for_linked_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_component_override_tracking");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    let old_ron = ron::to_string(
        &editor
            .game
            .ecs
            .get::<Name>(linked_root)
            .expect("linked instance should have a name"),
    )
    .expect("name should serialize");

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(UpdateComponentCmd::new(
        linked_root,
        EditorMode::Room(room_id),
        Name::TYPE_NAME,
        old_ron,
        "(\"Edited Root\")".to_string(),
        Default::default(),
        Default::default(),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let overrides = editor
            .game
            .ecs
            .get::<PrefabOverrides>(linked_root)
            .expect("linked instance edit should create prefab overrides");
        assert!(overrides
            .modified_components
            .iter()
            .any(|type_name| type_name == Name::TYPE_NAME));
    });
}
