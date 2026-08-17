use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Scales the world gravity applied to an entity.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct GravityScale(pub f32);

impl Default for GravityScale {
    fn default() -> Self {
        Self(1.0)
    }
}

inspector_module!(GravityScale);
