use engine_core::ecs::facing_direction::Direction;

pub(crate) fn parse_direction(value: &str) -> Result<Direction, String> {
    ron::de::from_str::<Direction>(value).map_err(|_| {
        format!(
            "Unsupported direction '{value}'. Expected one of: down, up, down_left, down_right, right, left, up_left, up_right."
        )
    })
}

pub(crate) fn flip_x_for_direction(direction: Direction) -> bool {
    direction.has_leftward_component()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::facing_direction::Direction;

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
    fn leftward_flip_helper_only_flips_for_leftward_directions() {
        assert!(flip_x_for_direction(Direction::Left));
        assert!(flip_x_for_direction(Direction::DownLeft));
        assert!(!flip_x_for_direction(Direction::Up));
        assert!(!flip_x_for_direction(Direction::Right));
    }
}
