use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Reflect)]
#[serde(default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
