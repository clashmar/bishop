use bishop::prelude::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

/// Returns a default room for collision-world tests.
pub(super) fn empty_room() -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, 8),
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Returns a world containing the default collision-world test room.
pub(super) fn empty_world() -> World {
    let room = empty_room();
    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}
