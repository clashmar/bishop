use super::*;
use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::comp_type_name;
use crate::ecs::entity::get_parent;
use crate::ecs::{CurrentFrame, CurrentRoom, Ecs, Entity, Name, Pivot, Transform, Velocity};
use crate::game::Game;
use crate::prefab::{PrefabAsset, PrefabId, PrefabNode};
use crate::prelude::Vec2;
use crate::scripting::script_manager::ScriptManager;
use crate::worlds::room::RoomId;
use crate::worlds::world::{World, WorldId};
use std::collections::HashSet;
use uuid::Uuid;

mod capture_tests;
mod instance_tests;

fn test_game() -> Game {
    let world_id = WorldId(Uuid::new_v4());
    Game {
        id: Uuid::new_v4(),
        name: "prefab_tests".to_string(),
        worlds: vec![World {
            id: world_id,
            ..Default::default()
        }],
        current_world_id: world_id,
        sprite_manager: SpriteManager::default(),
        script_manager: ScriptManager::default(),
        ..Default::default()
    }
}

fn find_entity_for_node(ecs: &Ecs, root_entity: Entity, node_id: usize) -> Option<Entity> {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .find_map(|(entity, metadata)| {
            (metadata.root_entity == root_entity && metadata.node_id == node_id).then_some(*entity)
        })
}
