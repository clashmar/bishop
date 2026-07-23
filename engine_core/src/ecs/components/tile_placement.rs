use crate::tiles::TileDefId;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

#[ecs_component]
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
