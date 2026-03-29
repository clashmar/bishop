// game/src/input/input_snapshot.rs
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
    /// Fill snapshot with the current input state from the platform context.
    pub fn capture_input_state(&mut self, ctx: &PlatformContext) {
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
}
