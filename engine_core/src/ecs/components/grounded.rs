use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

#[ecs_component]
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Grounded(#[serde(skip)] pub bool);
