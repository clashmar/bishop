use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marker trait for global components.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Global {}
