use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default, Reflect)]
pub struct Damage {
    pub amount: f32,
}
