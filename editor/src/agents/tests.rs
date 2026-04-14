use super::test_helpers::seeded_editor_fixture;
use super::{build_seeded_agent_payload, write_seeded_agent_payload};
use crate::app::Editor;
use crate::playtest::room_playtest::resolve_playtest_binary;
use engine_core::agents::payload::AgentPayloadError;
use engine_core::agents::visibility::AgentSnapshotRequest;
use engine_core::constants::agents;
use engine_core::ecs::component::ComponentStore;
use engine_core::payload;
use engine_core::prelude::*;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::any::TypeId;
use std::ffi::OsString;
use std::fs;

#[derive(Default)]
struct UnknownComponent;

fn seeded_editor_with_duplicate_names(test_game: &TestGameFolder, room_id: RoomId) -> Editor {
    let mut editor = seeded_editor_fixture(test_game, room_id);
    editor
        .game
        .get_world_mut(editor.game.current_world_id)
        .rooms
        .iter_mut()
        .find(|room| room.id == room_id)
        .unwrap()
        .variants
        .clear();

    editor
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(CurrentRoom(room_id))
        .with(Name("Duplicate Entity".to_string()))
        .finish();

    editor
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(CurrentRoom(room_id))
        .with(Name("Duplicate Entity".to_string()))
        .finish();

    editor
}

#[test]
fn seeded_agent_payload_preserves_current_game_and_room_identity() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_identity");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(7));
    let room_id = editor.cur_room_id.unwrap();

    let payload = build_seeded_agent_payload(&editor, room_id, None).unwrap();

    assert_eq!(payload.game.name, editor.game.name);
    assert_eq!(payload.room.id, room_id);
}

#[test]
fn seeded_agent_payload_preserves_default_snapshot_request() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_snapshot_request");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(8));
    let room_id = editor.cur_room_id.unwrap();
    let request = AgentSnapshotRequest {
        extras: payload!(player_velocity_x: 2.5),
    };

    let payload = build_seeded_agent_payload(&editor, room_id, Some(request.clone())).unwrap();

    assert_eq!(payload.snapshot_request, Some(request));
}

#[test]
fn seeded_agent_payload_rejects_missing_component_types() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_unknown_component");
    set_game_name(test_game.name());

    let mut editor = seeded_editor_fixture(&test_game, RoomId(11));
    editor.game.ecs.stores.insert(
        TypeId::of::<ComponentStore<UnknownComponent>>(),
        Box::new(ComponentStore::<UnknownComponent>::default()),
    );

    let room_id = editor.cur_room_id.unwrap();

    let result = build_seeded_agent_payload(&editor, room_id, None);

    assert!(matches!(
        result,
        Err(AgentPayloadError::UnknownComponentType(_))
    ));
}

#[test]
fn seeded_agent_payload_rejects_missing_room_id() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_missing_room");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(21));

    let result = build_seeded_agent_payload(&editor, RoomId(9999), None);

    assert!(matches!(result, Err(AgentPayloadError::MissingRoom)));
}

#[test]
fn seeded_agent_payload_rejects_duplicate_entity_names() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_duplicate_names");
    set_game_name(test_game.name());

    let editor = seeded_editor_with_duplicate_names(&test_game, RoomId(31));
    let room_id = editor.cur_room_id.unwrap();

    let result = build_seeded_agent_payload(&editor, room_id, None);

    assert!(matches!(
        result,
        Err(AgentPayloadError::DuplicateEntityName(name)) if name == "Duplicate Entity"
    ));
}

#[test]
fn seeded_agent_payload_can_round_trip_through_file_export() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_file_round_trip");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(41));
    let room_id = editor.cur_room_id.unwrap();

    let path = write_seeded_agent_payload(&editor, room_id, None).unwrap();
    let loaded = game_lib::agents::load_agent_payload(&path).unwrap();

    assert_eq!(loaded.game.name, editor.game.name);
    assert_eq!(loaded.room.id, room_id);

    fs::remove_file(path).unwrap();
}

#[test]
fn seeded_agent_payload_round_trips_snapshot_request_through_file_export() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_snapshot_file_round_trip");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(42));
    let room_id = editor.cur_room_id.unwrap();
    let request = AgentSnapshotRequest {
        extras: payload!(player_velocity_x: 2.5),
    };

    let path = write_seeded_agent_payload(&editor, room_id, Some(request.clone())).unwrap();
    let loaded = game_lib::agents::load_agent_payload(&path).unwrap();

    assert_eq!(loaded.snapshot_request, Some(request));

    fs::remove_file(path).unwrap();
}

#[test]
fn seeded_agent_payload_file_can_be_launched_with_canonical_args() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("agent_playtest_launch_builder");
    let editor = seeded_editor_fixture(&test_game, RoomId(51));
    let room_id = editor.cur_room_id.unwrap();

    let payload_path = write_seeded_agent_payload(&editor, room_id, None).unwrap();
    let exe_path = resolve_playtest_binary().unwrap();
    let args = [
        OsString::from(agents::HEADLESS_FLAG),
        OsString::from(agents::PAYLOAD_FLAG),
        payload_path.as_os_str().to_os_string(),
    ];

    assert_eq!(args[0], OsString::from(agents::HEADLESS_FLAG));
    assert_eq!(args[1], OsString::from(agents::PAYLOAD_FLAG));
    assert_eq!(args[2], payload_path.as_os_str());
    assert!(exe_path.exists());

    let _ = fs::remove_file(payload_path);
}

#[test]
fn seeded_editor_fixture_helper_supports_control_and_export_tests() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_fixture_helper");
    set_game_name(test_game.name());

    let editor = seeded_editor_fixture(&test_game, RoomId(52));

    assert_eq!(editor.cur_room_id, Some(RoomId(52)));
}
