use bishop::input::{GAMEPAD_AXIS_DEADZONE, GamepadState, Input, KeyCode};
use bishop::prelude::PlatformContext;
use engine_core::input::{AxisDirection, InputActionId, InputActionMap, InputBinding, InputSource};
use engine_core::input::input_table::{KEY_TABLE, MOUSE_TABLE};
use engine_core::storage::InputSettings;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default)]
struct ButtonState {
    down: bool,
    pressed: bool,
    released: bool,
}

#[derive(Clone, Default)]
pub(crate) struct PhysicalInputState {
    keys: HashMap<KeyCode, ButtonState>,
    gamepad: GamepadState,
}

impl PhysicalInputState {
    #[cfg(test)]
    fn set_key(&mut self, key: KeyCode, down: bool, pressed: bool, released: bool) {
        self.keys.insert(
            key,
            ButtonState {
                down,
                pressed,
                released,
            },
        );
    }

    #[cfg(test)]
    fn set_gamepad(&mut self, gamepad: GamepadState) {
        self.gamepad = gamepad;
    }

    fn state_for_binding(&self, binding: &InputBinding) -> ButtonState {
        match binding.source {
            InputSource::Key(key) => self.keys.get(&key).copied().unwrap_or_default(),
            InputSource::GamepadButton(button) => ButtonState {
                down: self.gamepad.button_down(button),
                pressed: self.gamepad.button_pressed(button),
                released: self.gamepad.button_released(button),
            },
            InputSource::GamepadAxis {
                axis,
                direction,
                threshold,
            } => {
                let value = self.gamepad.axis_value(axis);
                let threshold = threshold.max(GAMEPAD_AXIS_DEADZONE);
                let active = match direction {
                    AxisDirection::Positive => value >= threshold,
                    AxisDirection::Negative => value <= -threshold,
                };
                ButtonState {
                    down: active,
                    pressed: false,
                    released: false,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ActionState {
    down: bool,
    pressed: bool,
    released: bool,
}

#[derive(Clone, Default)]
pub struct InputSnapshot {
    pub down: HashMap<&'static str, bool>,
    pub pressed: HashMap<&'static str, bool>,
    pub released: HashMap<&'static str, bool>,
    actions: HashMap<InputActionId, ActionState>,
    game_id: Option<Uuid>,
}

impl InputSnapshot {
    /// Captures physical inputs and resolves the current game's named actions.
    pub fn capture_input_state(
        &mut self,
        ctx: &PlatformContext,
        actions: &InputActionMap,
        settings: &InputSettings,
        game_id: Uuid,
        gamepad: &GamepadState,
    ) {
        if self.game_id != Some(game_id) {
            self.actions.clear();
            self.game_id = Some(game_id);
        }
        let ctx = ctx.borrow();

        self.down.clear();
        self.pressed.clear();
        self.released.clear();

        let mut physical = PhysicalInputState {
            keys: HashMap::new(),
            gamepad: gamepad.clone(),
        };

        for &(name, code) in KEY_TABLE {
            let state = ButtonState {
                down: ctx.is_key_down(code),
                pressed: ctx.is_key_pressed(code),
                released: ctx.is_key_released(code),
            };
            self.down.insert(name, state.down);
            self.pressed.insert(name, state.pressed);
            self.released.insert(name, state.released);
            physical.keys.insert(code, state);
        }

        for &(name, button) in MOUSE_TABLE {
            self.down.insert(name, ctx.is_mouse_button_down(button));
            self.pressed
                .insert(name, ctx.is_mouse_button_pressed(button));
            self.released
                .insert(name, ctx.is_mouse_button_released(button));
        }

        self.resolve_actions(actions, settings, game_id, &physical);
    }

    /// Resolves all authored actions from one physical input state.
    pub(crate) fn resolve_actions(
        &mut self,
        actions: &InputActionMap,
        settings: &InputSettings,
        game_id: Uuid,
        physical: &PhysicalInputState,
    ) {
        let previous = std::mem::take(&mut self.actions);
        for (action_id, _) in &actions.actions {
            let was_down = previous.get(action_id).is_some_and(|state| state.down);
            let bindings = settings.bindings_for(game_id, action_id, actions);
            let mut down = false;
            let mut any_pressed = false;
            let mut any_released = false;

            for binding in bindings {
                let state = physical.state_for_binding(binding);
                down |= state.down;
                any_pressed |= state.pressed;
                any_released |= state.released;
            }

            self.actions.insert(
                action_id.clone(),
                ActionState {
                    down,
                    pressed: any_pressed || (down && !was_down),
                    released: !down && (any_released || was_down),
                },
            );
        }
    }

    /// Returns whether the named action is currently active.
    pub fn is_action_down(&self, action: &InputActionId) -> bool {
        self.actions.get(action).is_some_and(|state| state.down)
    }

    /// Returns whether the named action became active this frame.
    pub fn is_action_pressed(&self, action: &InputActionId) -> bool {
        self.actions.get(action).is_some_and(|state| state.pressed)
    }

    /// Returns whether the named action became inactive this frame.
    pub fn is_action_released(&self, action: &InputActionId) -> bool {
        self.actions.get(action).is_some_and(|state| state.released)
    }
}
