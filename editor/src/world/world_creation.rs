use engine_core::constants::world as world_constants;
use engine_core::ecs::{Name, PlayerProxy, Transform};
use engine_core::game::Game;
use engine_core::worlds::{Room, World, WorldMeta};

/// Create a fresh world with a single default room.
pub fn create_new_world(game: &mut Game) -> World {
    let id = game.id_allocator.allocate_world_id();
    let name = "new".to_string();
    let room_id = game.id_allocator.allocate_room_id();
    let first_room = Room::new(&mut game.ecs, room_id, world_constants::DEFAULT_GRID_SIZE);
    let room_origin = first_room.position;

    let mut world = World::new(id, name.clone(), world_constants::DEFAULT_GRID_SIZE);
    world.add_room(first_room);
    world.current_room_id = None;
    world.starting_room_id = Some(room_id);
    world.starting_position = Some(room_origin);
    world.meta = WorldMeta::default();

    let _spawn_point = game
        .ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: room_origin,
            ..Default::default()
        })
        .with(Name("Player Proxy".to_string()))
        .with_current_room(room_id)
        .finish();

    world
}
