use super::{Active, Collider, Grounded, MotionBody, Transform, Velocity};
use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marker for participation in the physics system.
#[ecs_component(deps = [Active, Collider, Grounded, MotionBody, Transform, Velocity])]
#[derive(Default, Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct PhysicsBody;

inspector_module!(PhysicsBody, removable = true, title = "Physics Body");
