// engine_core/src/ecs/facing_direction.rs
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Direction an entity is facing, used for auto-flip logic with mirrored clips.
#[ecs_component]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FacingDirection(pub Direction);

/// Facing direction with support for horizontal, vertical, and diagonal values.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Down,
    Up,
    DownLeft,
    DownRight,
    Right,
    Left,
    UpLeft,
    UpRight,
}

impl Direction {
    /// Returns true if the direction has a leftward horizontal component.
    pub fn has_leftward_component(&self) -> bool {
        matches!(self, Direction::Left | Direction::UpLeft | Direction::DownLeft)
    }

    /// Returns true if the direction has a rightward horizontal component.
    pub fn has_rightward_component(&self) -> bool {
        matches!(
            self,
            Direction::Right | Direction::UpRight | Direction::DownRight
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_deserializes_from_snake_case_values() {
        assert_eq!(ron::de::from_str::<Direction>("up_left").unwrap(), Direction::UpLeft);
        assert_eq!(
            ron::de::from_str::<Direction>("down_right").unwrap(),
            Direction::DownRight
        );
    }

    #[test]
    fn direction_serializes_to_snake_case_values() {
        assert_eq!(ron::to_string(&Direction::Up).unwrap(), "up");
        assert_eq!(ron::to_string(&Direction::DownLeft).unwrap(), "down_left");
    }

    #[test]
    fn direction_leftward_helper_matches_leftward_variants_only() {
        assert!(Direction::Left.has_leftward_component());
        assert!(Direction::UpLeft.has_leftward_component());
        assert!(Direction::DownLeft.has_leftward_component());
        assert!(!Direction::Up.has_leftward_component());
        assert!(!Direction::Right.has_leftward_component());
        assert!(!Direction::DownRight.has_leftward_component());
    }
}
