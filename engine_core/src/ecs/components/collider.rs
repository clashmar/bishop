use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
}
inspector_module!(Collider);

impl Default for Collider {
    fn default() -> Self {
        Self {
            width: 16.0,
            height: 16.0,
        }
    }
}
