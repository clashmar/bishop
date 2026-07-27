use crate::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::TilePlacement;
use crate::worlds::{RoomId, RoomLayer};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Component that stores the room and authored layer an entity belongs to.
#[ecs_component(on_insert = on_insert, on_remove = on_remove, guarded, lua_api = false)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct CurrentRoom {
    pub room_id: RoomId,
    pub layer: RoomLayer,
}

impl CurrentRoom {
    pub fn new(room_id: RoomId, layer: RoomLayer) -> Self {
        Self { room_id, layer }
    }

    pub fn front(room_id: RoomId) -> Self {
        Self::new(room_id, RoomLayer::Front)
    }
}

fn on_insert(comp: &mut CurrentRoom, entity: &Entity, ecs: &mut Ecs) {
    ecs.room_entities.entry(comp.room_id).or_default().insert(*entity);
    ecs.index_room_layer_entity(comp.room_id, comp.layer, *entity);
    if let Some(placement) = ecs.get::<TilePlacement>(*entity).copied() {
        ecs.index_tile_placement(comp.room_id, comp.layer, *entity, placement);
    }
}

fn on_remove(comp: &mut CurrentRoom, entity: &Entity, ecs: &mut Ecs) {
    if let Some(placement) = ecs.get::<TilePlacement>(*entity).copied() {
        ecs.unindex_tile_placement(comp.room_id, comp.layer, *entity, placement);
    }
    ecs.unindex_room_layer_entity(comp.room_id, comp.layer, *entity);
    if let Some(entities) = ecs.room_entities.get_mut(&comp.room_id) {
        entities.remove(entity);
        if entities.is_empty() {
            ecs.room_entities.remove(&comp.room_id);
        }
    }
}
