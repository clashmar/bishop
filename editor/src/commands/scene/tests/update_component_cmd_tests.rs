use bishop::prelude::Vec2;
use engine_core::animation::ClipId;
use engine_core::ecs::*;
use engine_core::engine_global::{set_game_name};
use engine_core::worlds::*;
use super::*;
use crate::editor_global::{reset_services, set_editor};

#[test]
fn collider_timeline_update_component_command_undoes_nested_frame_data() {
    reset_services();

    let mut editor = Editor::default();
    editor.game.add_world(Default::default());
    let entity = editor
        .game
        .ecs
        .create_entity()
        .with(Collider::default())
        .finish();

    let old_collider = Collider::default();
    let mut new_collider = Collider::default();
    new_collider.set_frame_data(
        ClipId::Run,
        ColliderFrameKey { row: 0, col: 1 },
        ColliderData {
            shape: ColliderShape::Circle { radius: 9.0 },
            offset: Vec2::new(4.0, 5.0),
        },
    );

    let old_ron = ron::to_string(&old_collider).expect("Collider should serialize");
    let new_ron = ron::to_string(&new_collider).expect("Collider should serialize");
    set_editor(editor);

    let mut cmd = UpdateComponentCmd::new(
        entity,
        EditorMode::Room(RoomId(1)),
        Collider::TYPE_NAME,
        old_ron,
        new_ron,
        Default::default(),
        Default::default(),
    );
    cmd.execute();

    with_editor(|editor| {
        let collider = editor.game.ecs.get::<Collider>(entity).expect("Collider should exist");
        assert_eq!(
            collider
                .effective_data_for_frame(&ClipId::Run, ColliderFrameKey { row: 0, col: 1 })
                .offset,
            Vec2::new(4.0, 5.0),
        );
    });

    cmd.undo();

    with_editor(|editor| {
        let collider = editor.game.ecs.get::<Collider>(entity).expect("Collider should exist");
        assert_eq!(
            collider.effective_data_for_frame(&ClipId::Run, ColliderFrameKey { row: 0, col: 1 }),
            collider.static_data(),
        );
    });
}

#[test]
fn collider_sync_update_component_command_redoes_nested_frame_data() {
    reset_services();

    let mut editor = Editor::default();
    editor.game.add_world(Default::default());
    let entity = editor
        .game
        .ecs
        .create_entity()
        .with(Collider::default())
        .finish();

    let old_collider = Collider::default();
    let mut new_collider = Collider::default();
    let frame = ColliderFrameKey { row: 0, col: 1 };
    let frame_data = ColliderData {
        shape: ColliderShape::Aabb {
            width: 32.0,
            height: 48.0,
        },
        offset: Vec2::new(4.0, 5.0),
    };
    new_collider.set_frame_data(ClipId::Run, frame, frame_data);

    let old_ron = ron::to_string(&old_collider).expect("Collider should serialize");
    let new_ron = ron::to_string(&new_collider).expect("Collider should serialize");
    set_editor(editor);

    let mut cmd = UpdateComponentCmd::new(
        entity,
        EditorMode::Room(RoomId(1)),
        Collider::TYPE_NAME,
        old_ron,
        new_ron,
        Default::default(),
        Default::default(),
    );

    cmd.execute();
    cmd.undo();
    cmd.execute();

    with_editor(|editor| {
        let collider = editor.game.ecs.get::<Collider>(entity).expect("Collider should exist");
        assert_eq!(collider.effective_data_for_frame(&ClipId::Run, frame), frame_data);
    });
}

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

    let old_ron = ron::to_string(&CurrentRoom::front(RoomId(1)))
        .expect("CurrentRoom should serialize");
    let new_ron = ron::to_string(&CurrentRoom::front(RoomId(2)))
        .expect("CurrentRoom should serialize");

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
        assert_eq!(
            editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.room_id),
            Some(RoomId(2))
        );
        assert!(!editor.game.ecs.entities_in_room(RoomId(1)).contains(&entity));
        assert!(editor.game.ecs.entities_in_room(RoomId(2)).contains(&entity));
    });

    cmd.undo();

    with_editor(|editor| {
        assert_eq!(
            editor.game.ecs.get::<CurrentRoom>(entity).map(|room| room.room_id),
            Some(RoomId(1))
        );
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
