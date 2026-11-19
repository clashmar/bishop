use crate::constants::world;
use serde::{Deserialize, Serialize};

/// Physics settings that apply to a world.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldPhysics {
    /// Gravity applied to bodies in grid units per second squared.
    pub gravity: f32,
}

impl Default for WorldPhysics {
    fn default() -> Self {
        Self {
            gravity: world::DEFAULT_WORLD_GRAVITY,
        }
    }
}
