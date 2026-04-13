use super::{build_seeded_agent_payload, write_seeded_agent_payload};
use crate::app::{Editor, EditorMode};
use crate::storage::editor_storage::create_new_game;
use engine_core::agents::payload::AgentPayloadError;
use engine_core::ecs::component::ComponentStore;
use engine_core::prelude::*;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::any::TypeId;
use std::fs;

#[derive(Default)]
struct UnknownComponent;

fn seeded_editor(test_game: &TestGameFolder, _room_id: RoomId) -> Editor {
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let world_id = game.current_world_id;
    let world = game.get_world_mut(world_id);
    let room = world
        .rooms
        .first_mut()
        .expect("new game should create an initial room");
    room.name = "Seeded Room".to_string();
    let room_id = room.id;
    world.current_room_id = Some(room_id);
    world.starting_room_id = Some(room_id);

    game.ecs
        .create_entity()
        .with(Transform::default())
        .with(CurrentRoom(room_id))
        .with(Name("Seeded Entity".to_string()))
        .finish();

    Editor {
        game,
        mode: EditorMode::Room(room_id),
        cur_world_id: Some(world_id),
        cur_room_id: Some(room_id),
        ..Default::default()
    }
}

fn seeded_editor_with_duplicate_names(test_game: &TestGameFolder, room_id: RoomId) -> Editor {
    let mut editor = seeded_editor(test_game, room_id);
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

    let editor = seeded_editor(&test_game, RoomId(7));
    let room_id = editor.cur_room_id.unwrap();

    let payload = build_seeded_agent_payload(&editor, room_id).unwrap();

    assert_eq!(payload.game.name, editor.game.name);
    assert_eq!(payload.room.id, room_id);
}

#[test]
fn seeded_agent_payload_rejects_missing_component_types() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_unknown_component");
    set_game_name(test_game.name());

    let mut editor = seeded_editor(&test_game, RoomId(11));
    editor.game.ecs.stores.insert(
        TypeId::of::<ComponentStore<UnknownComponent>>(),
        Box::new(ComponentStore::<UnknownComponent>::default()),
    );

    let room_id = editor.cur_room_id.unwrap();

    let result = build_seeded_agent_payload(&editor, room_id);

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

    let editor = seeded_editor(&test_game, RoomId(21));

    let result = build_seeded_agent_payload(&editor, RoomId(9999));

    assert!(matches!(result, Err(AgentPayloadError::MissingRoom)));
}

#[test]
fn seeded_agent_payload_rejects_duplicate_entity_names() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("editor_seeded_agent_payload_duplicate_names");
    set_game_name(test_game.name());

    let editor = seeded_editor_with_duplicate_names(&test_game, RoomId(31));
    let room_id = editor.cur_room_id.unwrap();

    let result = build_seeded_agent_payload(&editor, room_id);

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

    let editor = seeded_editor(&test_game, RoomId(41));
    let room_id = editor.cur_room_id.unwrap();

    let path = write_seeded_agent_payload(&editor, room_id).unwrap();
    let loaded = game_lib::agents::load_agent_payload(path.to_str().unwrap()).unwrap();

    assert_eq!(loaded.game.name, editor.game.name);
    assert_eq!(loaded.room.id, room_id);

    fs::remove_file(path).unwrap();
}
