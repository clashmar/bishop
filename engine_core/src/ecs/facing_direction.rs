// engine_core/src/ecs/facing_direction.rs
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Direction an entity is facing, used for auto-flip logic with mirrored clips.
#[ecs_component]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FacingDirection(pub Direction);

/// Left or right facing direction.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Right,
    Left,
}

impl Direction {
    /// Returns true if facing left.
    pub fn is_left(&self) -> bool {
        matches!(self, Direction::Left)
    }

    /// Returns true if facing right.
    pub fn is_right(&self) -> bool {
        matches!(self, Direction::Right)
    }
}
