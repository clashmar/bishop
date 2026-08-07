use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct Solid(pub bool);

impl Default for Solid {
    fn default() -> Self {
        Self(true)
    }
}

inspector_module!(Solid);
