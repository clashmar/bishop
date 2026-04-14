use crate::app::{Editor, EditorMode};
use crate::storage::editor_storage::create_new_game;
use engine_core::engine_global::set_game_name;
use engine_core::prelude::*;
use engine_core::storage::test_utils::TestGameFolder;
use std::path::{Path, PathBuf};

fn replace_seeded_room(game: &mut Game, room_id: RoomId) {
    let world_id = game.current_world_id;
    let (original_room_id, grid_size) = {
        let world = game.get_world_mut(world_id);
        (world.starting_room_id.unwrap(), world.grid_size)
    };

    let mut room = Room::new(&mut game.ecs, room_id, grid_size);
    room.name = "Seeded Room".to_string();

    let world = game.get_world_mut(world_id);
    world.rooms.clear();
    world.rooms.push(room);
    world.current_room_id = Some(room_id);
    world.starting_room_id = Some(room_id);
    game.next_room_id = game.next_room_id.max(room_id.0);

    if let Some(proxy) = game.ecs.get_player_proxy(original_room_id) {
        game.ecs
            .add_component_to_entity(proxy, CurrentRoom(room_id));
    }
}

pub(crate) fn seeded_editor_fixture(test_game: &TestGameFolder, room_id: RoomId) -> Editor {
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let world_id = game.current_world_id;
    replace_seeded_room(&mut game, room_id);

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

pub(crate) fn seeded_agent_session_dir(payload_path: &Path) -> PathBuf {
    payload_path.parent().unwrap().join(format!(
        "{}_agent",
        payload_path.file_stem().unwrap().to_string_lossy()
    ))
}
