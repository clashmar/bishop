use super::{Grounded, MotionBody};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marker for participation in the physics system.
#[ecs_component(deps = [Grounded, MotionBody])]
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicsBody;
