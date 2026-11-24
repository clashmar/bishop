use super::{InputActionId, InputActionMap, InputBinding};
use crate::input::{AxisDirection, GamepadAxis, InputSource};
use bishop::{GamepadButton, KeyCode};
use std::collections::HashMap;

const JUMP_ACTION: &str = "jump";

#[test]
fn input_action_map_defaults_to_no_actions() {
    let action_map = InputActionMap::default();

    assert!(action_map.actions.is_empty());
}

#[test]
fn input_action_map_round_trips_keyboard_button_and_axis_bindings() {
    let action_map = InputActionMap {
        actions: HashMap::from([(
            InputActionId::new(JUMP_ACTION),
            vec![
                InputBinding {
                    source: InputSource::Key(KeyCode::Space),
                },
                InputBinding {
                    source: InputSource::GamepadButton(GamepadButton::South),
                },
                InputBinding {
                    source: InputSource::GamepadAxis {
                        axis: GamepadAxis::LeftY,
                        direction: AxisDirection::Negative,
                        threshold: 0.6,
                    },
                },
            ],
        )]),
    };
    let encoded = ron::to_string(&action_map);
    assert!(encoded.is_ok());

    let decoded = encoded
        .ok()
        .and_then(|value| ron::from_str::<InputActionMap>(&value).ok());

    assert_eq!(decoded, Some(action_map));
}

#[test]
fn game_defaults_to_an_empty_authored_input_action_map() {
    let game = crate::game::Game::default();

    assert!(game.input_actions.actions.is_empty());
}
