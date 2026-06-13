use crate::worlds::world::{WorldExitTrigger, WorldId, WorldTransitionMode};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Authoring component: declares a world transition, fired by the engine based on the trigger type.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct WorldExit {
    /// Destination world; `None` means unconfigured.
    pub destination_world: Option<WorldId>,
    /// `WorldEntry` name in the destination world; `None` arrives at the world start.
    pub entry: Option<String>,
    /// Transport (move the player) or Activate (switch world in place).
    pub mode: WorldTransitionMode,
    /// What event fires this exit.
    pub trigger: WorldExitTrigger,
}
