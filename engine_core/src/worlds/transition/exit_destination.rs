use super::super::world::WorldId;
use serde::{Deserialize, Serialize};

/// Where a `WorldExit` sends the player or world switch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExitDestination {
    /// Target a specific world by stable id.
    World(WorldId),
    /// Pop the overlay stack and return to the caller.
    Return,
}

impl ExitDestination {
    /// Display label for the Return variant.
    pub const RETURN_LABEL: &'static str = "Return to Caller";
}

impl std::fmt::Display for ExitDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitDestination::World(_) => write!(f, "World"),
            ExitDestination::Return => write!(f, "{}", Self::RETURN_LABEL),
        }
    }
}
