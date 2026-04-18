use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Z layer of an entity.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Reflect)]
#[serde(default)]
pub struct Layer {
    pub z: i32,
}
inspector_module!(Layer);
