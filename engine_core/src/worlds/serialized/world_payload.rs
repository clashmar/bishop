use crate::ecs::Entity;
use crate::worlds::World;
use serde::{Deserialize, Serialize};

/// Heavy world-scoped payload data that does not belong to a room.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldPayload {
    /// World singleton entity snapshot, if present.
    pub singleton: Option<Entity>,
}

impl WorldPayload {
    pub fn capture(world: &World) -> Self {
        Self {
            singleton: world.singleton,
        }
    }

    pub fn apply(self, world: &mut World) {
        world.singleton = self.singleton;
    }

    pub fn clear(world: &mut World) {
        world.singleton = None;
    }
}
