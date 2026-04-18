use super::SubPixel;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marker for entities that participate in fixed-step movement.
#[ecs_component(lua_api = false, deps = [SubPixel])]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct MotionBody;
