use super::{Collider, MotionBody, Velocity};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marker component for the player entity.
#[ecs_component(deps = [Collider, Velocity, MotionBody])]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Player;
