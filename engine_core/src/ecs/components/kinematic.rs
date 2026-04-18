use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct Kinematic {}
