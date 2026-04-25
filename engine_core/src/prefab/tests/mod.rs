use super::*;
use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::comp_type_name;
use crate::ecs::entity::get_parent;
use crate::ecs::{CurrentFrame, CurrentRoom, Ecs, Entity, Name, Pivot, Transform, Velocity};
use crate::game::{Game, IdAllocator};
use crate::prefab::{PrefabAsset, PrefabId, PrefabNode};
use crate::prelude::Vec2;
use crate::scripting::script_manager::ScriptManager;
use crate::worlds::room::Room;
use crate::worlds::room::RoomId;
use crate::worlds::world::{World, WorldId};
use std::collections::HashSet;

mod capture_tests;
mod instance_tests;

fn test_game() -> Game {
    let mut game = Game::default();
    let world_id = game.id_allocator.allocate_world_id();
    let room_id = game.id_allocator.allocate_room_id();
    let world = World {
        id: world_id,
        rooms: vec![Room {
            id: room_id,
            ..Default::default()
        }],
        ..Default::default()
    };
    game.add_world(world);
    game
}

fn find_entity_for_node(ecs: &Ecs, root_entity: Entity, node_id: usize) -> Option<Entity> {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .find_map(|(entity, metadata)| {
            (metadata.root_entity == root_entity && metadata.node_id == node_id).then_some(*entity)
        })
}
