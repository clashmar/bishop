// game/src/input/input_snapshot.rs
use crate::input::VirtualInputState;
use bishop::prelude::*;
use engine_core::input::input_table::*;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct InputSnapshot {
    pub down: HashMap<&'static str, bool>,
    pub pressed: HashMap<&'static str, bool>,
    pub released: HashMap<&'static str, bool>,
}

impl InputSnapshot {
    /// Fill snapshot with raw platform input state only.
    pub fn capture_platform_input_state(&mut self, ctx: &PlatformContext) {
        let ctx = ctx.borrow();

        // Clear previous frame data
        self.down.clear();
        self.pressed.clear();
        self.released.clear();

        // Keyboard
        for &(name, code) in KEY_TABLE {
            self.down.insert(name, ctx.is_key_down(code));
            self.pressed.insert(name, ctx.is_key_pressed(code));
            self.released.insert(name, ctx.is_key_released(code));
        }

        // Mouse
        for &(name, button) in MOUSE_TABLE {
            self.down.insert(name, ctx.is_mouse_button_down(button));
            self.pressed
                .insert(name, ctx.is_mouse_button_pressed(button));
            self.released
                .insert(name, ctx.is_mouse_button_released(button));
        }
    }

    /// Fill snapshot with platform input plus a virtual-input overlay.
    pub fn capture_effective_input_state(
        &mut self,
        ctx: &PlatformContext,
        virtual_input: &VirtualInputState,
    ) {
        self.capture_platform_input_state(ctx);
        self.apply_virtual_input(virtual_input);
    }

    /// Overlays virtual input onto the captured platform snapshot.
    pub fn apply_virtual_input(&mut self, virtual_input: &VirtualInputState) {
        for (&name, &value) in virtual_input.down() {
            self.down.insert(name, value);
        }

        for (&name, &value) in virtual_input.pressed() {
            self.pressed.insert(name, value);
            self.released.insert(name, !value);
        }

        for (&name, &value) in virtual_input.released() {
            self.released.insert(name, value);
            self.pressed.insert(name, !value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_global::{
        clear_virtual_input_state, get_virtual_input_state, set_virtual_input_state,
    };
    use engine_core::input::input_constants;

    #[test]
    fn virtual_input_marks_right_as_down_when_active() {
        let mut snapshot = InputSnapshot::default();
        let virtual_input =
            crate::input::VirtualInputState::from_down_inputs([input_constants::RIGHT]);

        snapshot.down.insert(input_constants::RIGHT, false);

        snapshot.apply_virtual_input(&virtual_input);

        assert_eq!(snapshot.down.get(input_constants::RIGHT), Some(&true));
    }

    #[test]
    fn virtual_input_marks_space_as_pressed_when_active() {
        let mut snapshot = InputSnapshot::default();
        let virtual_input =
            crate::input::VirtualInputState::single_frame_press(input_constants::SPACE);

        snapshot.pressed.insert(input_constants::SPACE, false);
        snapshot.released.insert(input_constants::SPACE, true);

        snapshot.apply_virtual_input(&virtual_input);

        assert_eq!(snapshot.pressed.get(input_constants::SPACE), Some(&true));
        assert_eq!(snapshot.released.get(input_constants::SPACE), Some(&false));
    }

    #[test]
    fn virtual_input_pressed_and_released_override_platform_defaults() {
        let mut snapshot = InputSnapshot::default();
        let pressed = crate::input::VirtualInputState::single_frame_press(input_constants::SPACE);

        snapshot.pressed.insert(input_constants::SPACE, false);
        snapshot.released.insert(input_constants::SPACE, true);
        snapshot.apply_virtual_input(&pressed);

        assert_eq!(snapshot.pressed.get(input_constants::SPACE), Some(&true));
        assert_eq!(snapshot.released.get(input_constants::SPACE), Some(&false));

        let released =
            crate::input::VirtualInputState::single_frame_release(input_constants::LEFT_SHIFT);

        snapshot.down.insert(input_constants::LEFT_SHIFT, true);
        snapshot.pressed.insert(input_constants::LEFT_SHIFT, true);
        snapshot.released.insert(input_constants::LEFT_SHIFT, false);
        snapshot.apply_virtual_input(&released);

        assert_eq!(snapshot.down.get(input_constants::LEFT_SHIFT), Some(&false));
        assert_eq!(
            snapshot.pressed.get(input_constants::LEFT_SHIFT),
            Some(&false)
        );
        assert_eq!(
            snapshot.released.get(input_constants::LEFT_SHIFT),
            Some(&true)
        );
    }

    #[test]
    fn virtual_input_marks_left_shift_as_released_and_not_down_when_active() {
        let mut snapshot = InputSnapshot::default();
        let virtual_input =
            crate::input::VirtualInputState::single_frame_release(input_constants::LEFT_SHIFT);

        snapshot.down.insert(input_constants::LEFT_SHIFT, true);
        snapshot.pressed.insert(input_constants::LEFT_SHIFT, true);
        snapshot.released.insert(input_constants::LEFT_SHIFT, false);

        snapshot.apply_virtual_input(&virtual_input);

        assert_eq!(snapshot.down.get(input_constants::LEFT_SHIFT), Some(&false));
        assert_eq!(
            snapshot.pressed.get(input_constants::LEFT_SHIFT),
            Some(&false)
        );
        assert_eq!(
            snapshot.released.get(input_constants::LEFT_SHIFT),
            Some(&true)
        );
    }

    struct VirtualInputCleanupGuard;

    impl VirtualInputCleanupGuard {
        fn new() -> Self {
            clear_virtual_input_state();
            Self
        }
    }

    impl Drop for VirtualInputCleanupGuard {
        fn drop(&mut self) {
            clear_virtual_input_state();
        }
    }

    #[test]
    fn virtual_input_state_round_trips_through_game_services() {
        let _guard = VirtualInputCleanupGuard::new();

        set_virtual_input_state(crate::input::VirtualInputState::single_frame_release(
            input_constants::LEFT_SHIFT,
        ));
        let virtual_input = get_virtual_input_state();

        assert_eq!(
            virtual_input.released().get(input_constants::LEFT_SHIFT),
            Some(&true)
        );
    }

    #[test]
    fn clearing_virtual_edges_preserves_held_down_inputs() {
        let mut virtual_input =
            crate::input::VirtualInputState::from_down_inputs([input_constants::RIGHT]);

        virtual_input.clear_edges();

        assert_eq!(
            virtual_input.down().get(input_constants::RIGHT),
            Some(&true)
        );
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }

    #[test]
    fn clearing_virtual_edges_removes_transient_press_down_override() {
        let mut virtual_input =
            crate::input::VirtualInputState::single_frame_press(input_constants::RIGHT);

        virtual_input.clear_edges();

        assert_eq!(virtual_input.down().get(input_constants::RIGHT), None);
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }

    #[test]
    fn clearing_virtual_edges_removes_transient_release_down_override() {
        let mut virtual_input =
            crate::input::VirtualInputState::single_frame_release(input_constants::LEFT_SHIFT);

        virtual_input.clear_edges();

        assert_eq!(virtual_input.down().get(input_constants::LEFT_SHIFT), None);
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }

    #[test]
    fn clearing_virtual_input_edges_in_game_services_preserves_down_state() {
        let _guard = VirtualInputCleanupGuard::new();

        set_virtual_input_state(crate::input::VirtualInputState::from_down_inputs([
            input_constants::RIGHT,
        ]));
        crate::game_global::clear_virtual_input_edges();
        let virtual_input = get_virtual_input_state();

        assert_eq!(
            virtual_input.down().get(input_constants::RIGHT),
            Some(&true)
        );
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }

    #[test]
    fn clearing_virtual_input_edges_in_game_services_removes_press_down_override() {
        let _guard = VirtualInputCleanupGuard::new();

        set_virtual_input_state(crate::input::VirtualInputState::single_frame_press(
            input_constants::RIGHT,
        ));
        crate::game_global::clear_virtual_input_edges();
        let virtual_input = get_virtual_input_state();

        assert_eq!(virtual_input.down().get(input_constants::RIGHT), None);
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }

    #[test]
    fn clearing_virtual_input_edges_in_game_services_removes_release_down_override() {
        let _guard = VirtualInputCleanupGuard::new();

        set_virtual_input_state(crate::input::VirtualInputState::single_frame_release(
            input_constants::LEFT_SHIFT,
        ));
        crate::game_global::clear_virtual_input_edges();
        let virtual_input = get_virtual_input_state();

        assert_eq!(virtual_input.down().get(input_constants::LEFT_SHIFT), None);
        assert!(virtual_input.pressed().is_empty());
        assert!(virtual_input.released().is_empty());
    }
}
