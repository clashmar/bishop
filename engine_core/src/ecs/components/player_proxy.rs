use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marker component for player proxies in rooms.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PlayerProxy;
