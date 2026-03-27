//! Input state tracking for wgpu backend.

use crate::input::{KeyCode, MouseButton};
use std::collections::HashSet;

/// Tracks keyboard and mouse input state per-frame.
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    mouse_position_prev: (f32, f32),
    mouse_wheel: (f32, f32),
    char_buffer: Vec<char>,
}

impl InputState {
    /// Creates a new input state with all state cleared.
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_pressed: HashSet::new(),
            mouse_released: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_position_prev: (0.0, 0.0),
            mouse_wheel: (0.0, 0.0),
            char_buffer: Vec::new(),
        }
    }

    /// Resets continuous state for new frame.
    pub fn begin_frame(&mut self) {
        // mouse_position_prev is updated at end_frame, not here
    }

    /// Handles a key press event.
    pub fn on_key_down(&mut self, key: KeyCode) {
        if self.keys_down.insert(key) {
            self.keys_pressed.insert(key);
        }
    }

    /// Handles a key release event.
    pub fn on_key_up(&mut self, key: KeyCode) {
        if self.keys_down.remove(&key) {
            self.keys_released.insert(key);
        }
    }

    /// Handles a mouse button press event.
    pub fn on_mouse_down(&mut self, button: MouseButton) {
        if self.mouse_down.insert(button) {
            self.mouse_pressed.insert(button);
        }
    }

    /// Handles a mouse button release event.
    pub fn on_mouse_up(&mut self, button: MouseButton) {
        if self.mouse_down.remove(&button) {
            self.mouse_released.insert(button);
        }
    }

    /// Handles a mouse move event.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Handles a mouse wheel event, accumulating delta for the frame.
    pub fn on_mouse_wheel(&mut self, delta_x: f32, delta_y: f32) {
        self.mouse_wheel.0 += delta_x;
        self.mouse_wheel.1 += delta_y;
    }

    /// Handles a character input event.
    pub fn on_char(&mut self, c: char) {
        self.char_buffer.push(c);
    }

    /// Returns true if the key is currently held down.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns true if the key was pressed this frame.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Returns true if the key was released this frame.
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Returns true if any key was pressed this frame.
    pub fn any_key_pressed(&self) -> bool {
        !self.keys_pressed.is_empty()
    }

    /// Returns true if the mouse button is currently held down.
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    /// Returns true if the mouse button was pressed this frame.
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    /// Returns true if the mouse button was released this frame.
    pub fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.mouse_released.contains(&button)
    }

    /// Returns the current mouse position.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Returns the mouse position delta since the last frame.
    pub fn mouse_delta_position(&self) -> (f32, f32) {
        (
            self.mouse_position.0 - self.mouse_position_prev.0,
            self.mouse_position.1 - self.mouse_position_prev.1,
        )
    }

    /// Returns the accumulated mouse wheel delta for this frame.
    pub fn mouse_wheel(&self) -> (f32, f32) {
        self.mouse_wheel
    }

    /// Returns characters typed this frame.
    pub fn chars_pressed(&self) -> Vec<char> {
        self.char_buffer.clone()
    }

    /// Clears per-frame state at end of frame.
    pub fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.mouse_wheel = (0.0, 0.0);
        self.char_buffer.clear();
        self.mouse_position_prev = self.mouse_position;
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
