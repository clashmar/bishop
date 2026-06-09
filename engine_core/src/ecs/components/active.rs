use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marks an entity as active for simulation systems.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct Active(pub bool);

impl Default for Active {
    fn default() -> Self {
        Self(true)
    }
}

inspector_module!(Active);
