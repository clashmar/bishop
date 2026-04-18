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
        matches!(
            self,
            Direction::Left | Direction::UpLeft | Direction::DownLeft
        )
    }

    /// Returns true if the direction has a rightward horizontal component.
    pub fn has_rightward_component(&self) -> bool {
        matches!(
            self,
            Direction::Right | Direction::UpRight | Direction::DownRight
        )
    }
}

pub fn parse_direction(value: &str) -> Result<Direction, String> {
    ron::de::from_str::<Direction>(value).map_err(|_| {
        format!(
            "Unsupported direction '{value}'. Expected one of: down, up, down_left, down_right, right, left, up_left, up_right."
        )
    })
}

pub fn flip_x_for_direction(direction: Direction) -> bool {
    direction.has_leftward_component()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_deserializes_from_snake_case_values() {
        assert_eq!(
            ron::de::from_str::<Direction>("up_left").unwrap(),
            Direction::UpLeft
        );
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

    #[test]
    fn parse_direction_accepts_canonical_direction_strings() {
        let directions = [
            Direction::Down,
            Direction::Up,
            Direction::DownLeft,
            Direction::DownRight,
            Direction::Right,
            Direction::Left,
            Direction::UpLeft,
            Direction::UpRight,
        ];

        for direction in directions {
            let canonical = ron::to_string(&direction).unwrap();
            assert_eq!(parse_direction(&canonical).unwrap(), direction);
        }
    }

    #[test]
    fn parse_direction_rejects_unknown_values() {
        assert!(parse_direction("north").is_err());
        assert!(parse_direction("upleft").is_err());
    }

    #[test]
    fn flip_x_helper_only_flips_for_leftward_directions() {
        assert!(flip_x_for_direction(Direction::Left));
        assert!(flip_x_for_direction(Direction::DownLeft));
        assert!(!flip_x_for_direction(Direction::Up));
        assert!(!flip_x_for_direction(Direction::Right));
    }
}
