use crate::input::{GamepadAxis, GamepadButton, GamepadState};
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};

pub(crate) struct GamepadPoller {
    gilrs: Option<Gilrs>,
    active_gamepad: Option<GamepadId>,
    state: GamepadState,
}

impl GamepadPoller {
    pub(crate) fn new() -> Self {
        match Gilrs::new() {
            Ok(gilrs) => {
                let mut poller = Self {
                    gilrs: Some(gilrs),
                    active_gamepad: None,
                    state: GamepadState::default(),
                };
                if let Some(gilrs) = poller.gilrs.as_ref() {
                    Self::select_active_gamepad(
                        gilrs,
                        &mut poller.active_gamepad,
                        &mut poller.state,
                    );
                }
                poller
            }
            Err(error) => {
                eprintln!("Could not initialize gamepad polling: {error}");
                Self {
                    gilrs: None,
                    active_gamepad: None,
                    state: GamepadState::default(),
                }
            }
        }
    }

    pub(crate) fn begin_frame(&mut self) {
        self.state.begin_frame();
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };
        while let Some(event) = gilrs.next_event() {
            Self::handle_event(gilrs, &mut self.active_gamepad, &mut self.state, event);
        }
    }

    pub(crate) fn state(&self) -> &GamepadState {
        &self.state
    }

    fn handle_event(
        gilrs: &Gilrs,
        active_gamepad: &mut Option<GamepadId>,
        state: &mut GamepadState,
        event: Event,
    ) {
        match event.event {
            EventType::Connected if active_gamepad.is_none() => {
                Self::select_active_gamepad(gilrs, active_gamepad, state);
            }
            EventType::Disconnected if *active_gamepad == Some(event.id) => {
                state.disconnect();
                *active_gamepad = None;
                Self::select_active_gamepad(gilrs, active_gamepad, state);
            }
            EventType::ButtonPressed(..)
            | EventType::ButtonReleased(..)
            | EventType::ButtonChanged(..)
            | EventType::AxisChanged(..)
                if *active_gamepad == Some(event.id) =>
            {
                Self::sync_state(gilrs, event.id, state);
            }
            _ => {}
        }
    }

    fn select_active_gamepad(
        gilrs: &Gilrs,
        active_gamepad: &mut Option<GamepadId>,
        state: &mut GamepadState,
    ) {
        let next = gilrs.gamepads().next().map(|(id, _)| id);
        *active_gamepad = next;
        state.disconnect();
        if let Some(id) = next {
            Self::sync_state(gilrs, id, state);
        }
    }

    fn sync_state(gilrs: &Gilrs, id: GamepadId, state: &mut GamepadState) {
        let gamepad = gilrs.gamepad(id);
        for (source, target) in button_mappings() {
            state.set_button(target, gamepad.is_pressed(source));
        }
        state.set_button(
            GamepadButton::LeftTrigger,
            gamepad.is_pressed(Button::LeftTrigger2),
        );
        state.set_button(
            GamepadButton::RightTrigger,
            gamepad.is_pressed(Button::RightTrigger2),
        );
        for (source, target) in axis_mappings() {
            state.set_axis_value(target, gamepad.value(source));
        }
        state.set_axis_value(
            GamepadAxis::LeftTrigger,
            trigger_value(&gamepad, Button::LeftTrigger2),
        );
        state.set_axis_value(
            GamepadAxis::RightTrigger,
            trigger_value(&gamepad, Button::RightTrigger2),
        );
    }
}

fn button_mappings() -> [(Button, GamepadButton); 14] {
    [
        (Button::South, GamepadButton::South),
        (Button::East, GamepadButton::East),
        (Button::North, GamepadButton::North),
        (Button::West, GamepadButton::West),
        (Button::LeftTrigger, GamepadButton::LeftShoulder),
        (Button::RightTrigger, GamepadButton::RightShoulder),
        (Button::Select, GamepadButton::Select),
        (Button::Start, GamepadButton::Start),
        (Button::LeftThumb, GamepadButton::LeftStick),
        (Button::RightThumb, GamepadButton::RightStick),
        (Button::DPadUp, GamepadButton::DPadUp),
        (Button::DPadDown, GamepadButton::DPadDown),
        (Button::DPadLeft, GamepadButton::DPadLeft),
        (Button::DPadRight, GamepadButton::DPadRight),
    ]
}

fn axis_mappings() -> [(Axis, GamepadAxis); 4] {
    [
        (Axis::LeftStickX, GamepadAxis::LeftX),
        (Axis::LeftStickY, GamepadAxis::LeftY),
        (Axis::RightStickX, GamepadAxis::RightX),
        (Axis::RightStickY, GamepadAxis::RightY),
    ]
}

fn trigger_value(gamepad: &gilrs::Gamepad<'_>, button: Button) -> f32 {
    gamepad.button_data(button).map_or(0.0, |data| data.value())
}
