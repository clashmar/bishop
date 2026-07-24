use crate::ecs::{CurrentRoom, Ecs};
use crate::ecs::entity::Entity;
use crate::tiles::TileDefId;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

#[ecs_component(on_insert = on_insert, on_remove = on_remove)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TilePlacement {
    pub definition: TileDefId,
    pub grid_x: usize,
    pub grid_y: usize,
}

impl Default for TilePlacement {
    fn default() -> Self {
        Self {
            definition: TileDefId(0),
            grid_x: 0,
            grid_y: 0,
        }
    }
}

impl TilePlacement {
    pub fn new(definition: TileDefId, grid_x: usize, grid_y: usize) -> Self {
        Self {
            definition,
            grid_x,
            grid_y,
        }
    }
}

fn on_insert(comp: &mut TilePlacement, entity: &Entity, ecs: &mut Ecs) {
    if let Some(room_id) = ecs.get::<CurrentRoom>(*entity).map(|room| room.0) {
        ecs.index_tile_placement(room_id, *entity, *comp);
    }
}

fn on_remove(comp: &mut TilePlacement, entity: &Entity, ecs: &mut Ecs) {
    if let Some(room_id) = ecs.get::<CurrentRoom>(*entity).map(|room| room.0) {
        ecs.unindex_tile_placement(room_id, *entity, *comp);
    }
}
