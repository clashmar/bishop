use crate::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::worlds::room::RoomId;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Component that stores the room identifier an entity belongs to.
#[ecs_component(on_insert = on_insert, on_remove = on_remove, guarded, lua_api = false)]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CurrentRoom(pub RoomId);

fn on_insert(comp: &mut CurrentRoom, entity: &Entity, ecs: &mut Ecs) {
    ecs.room_entities.entry(comp.0).or_default().insert(*entity);
}

fn on_remove(comp: &mut CurrentRoom, entity: &Entity, ecs: &mut Ecs) {
    if let Some(entities) = ecs.room_entities.get_mut(&comp.0) {
        entities.remove(entity);
        if entities.is_empty() {
            ecs.room_entities.remove(&comp.0);
        }
    }
}
