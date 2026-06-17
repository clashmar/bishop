use crate::worlds::exit_destination::ExitDestination;
use crate::worlds::world::WorldExitTrigger;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Authoring component: declares a world transition, fired by the engine based on the trigger type.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct WorldExit {
    /// Destination world or return action; `None` means unconfigured.
    pub destination: Option<ExitDestination>,
    /// `WorldEntry` name in the destination world; `None` arrives at the world start.
    pub entry: Option<String>,
    /// What event fires this exit.
    pub trigger: WorldExitTrigger,
}
