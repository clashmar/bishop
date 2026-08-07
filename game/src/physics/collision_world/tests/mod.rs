use super::*;
use engine_core::game::Game;
use engine_core::tiles::{TileMap, TileRegistry, apply_tile_definition_to_entity, tile_definition_component_snapshot};

use super::test_support::{empty_room, empty_world};

fn world_with_room(room: Room) -> World {
    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn room_with_back_zones(interior_zones: Vec<InteriorZone>) -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: Vec2::new(4.0, 4.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(4, 4),
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    interior_zones,
                    ..Default::default()
                }),
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

mod entity_solid_tests;
mod layer_tests;
mod overlap_tests;
