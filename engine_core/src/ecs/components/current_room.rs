use crate::worlds::room::RoomId;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Component that stores the room identifier an entity belongs to.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CurrentRoom(pub RoomId);
