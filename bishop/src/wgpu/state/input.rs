//! Input state tracking for wgpu backend.

use crate::input::{KeyCode, MouseButton, MouseWheelKind};
use std::collections::HashSet;

/// Tracks keyboard and mouse input state per-frame.
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    mouse_double_clicked: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    mouse_position_prev: (f32, f32),
    mouse_wheel: (f32, f32),
    mouse_wheel_kind: Option<MouseWheelKind>,
    char_buffer: Vec<char>,
    last_click_times: [Option<f64>; 3],
    last_click_positions: [Option<(f32, f32)>; 3],
}

const DOUBLE_CLICK_THRESHOLD: f64 = 0.3;
const DOUBLE_CLICK_POSITION_TOLERANCE: f32 = 5.0;

fn mouse_button_index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
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
            mouse_double_clicked: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_position_prev: (0.0, 0.0),
            mouse_wheel: (0.0, 0.0),
            mouse_wheel_kind: None,
            char_buffer: Vec::new(),
            last_click_times: [None; 3],
            last_click_positions: [None; 3],
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
    pub fn on_mouse_down(&mut self, button: MouseButton, time: f64, pos: (f32, f32)) {
        if self.mouse_down.insert(button) {
            self.mouse_pressed.insert(button);

            let idx = mouse_button_index(button);
            if !crate::input::take_double_click_reset_requested() {
                if let Some((last_time, last_pos)) =
                    self.last_click_times[idx].zip(self.last_click_positions[idx])
                {
                    let dt = time - last_time;
                    let dx = pos.0 - last_pos.0;
                    let dy = pos.1 - last_pos.1;
                    if dt <= DOUBLE_CLICK_THRESHOLD
                        && dx.abs() <= DOUBLE_CLICK_POSITION_TOLERANCE
                        && dy.abs() <= DOUBLE_CLICK_POSITION_TOLERANCE
                    {
                        self.mouse_double_clicked.insert(button);
                    }
                }
            }
            self.last_click_times[idx] = Some(time);
            self.last_click_positions[idx] = Some(pos);
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
    pub fn on_mouse_wheel(&mut self, delta_x: f32, delta_y: f32, kind: MouseWheelKind) {
        self.mouse_wheel.0 += delta_x;
        self.mouse_wheel.1 += delta_y;
        self.mouse_wheel_kind = Some(kind);
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

    /// Returns true if the mouse button was double-clicked this frame.
    pub fn is_mouse_button_double_clicked(&self, button: MouseButton) -> bool {
        self.mouse_double_clicked.contains(&button)
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

    /// Returns the source kind for the most recent mouse wheel event this frame.
    pub fn mouse_wheel_kind(&self) -> Option<MouseWheelKind> {
        self.mouse_wheel_kind
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
        self.mouse_double_clicked.clear();
        self.mouse_wheel = (0.0, 0.0);
        self.mouse_wheel_kind = None;
        self.char_buffer.clear();
        self.mouse_position_prev = self.mouse_position;
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_click_is_not_double_click() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Left));
    }

    #[test]
    fn two_clicks_within_threshold_is_double_click() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Left, 0.1, (0.0, 0.0));
        assert!(input.is_mouse_button_double_clicked(MouseButton::Left));
    }

    #[test]
    fn two_clicks_beyond_threshold_is_not_double_click() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Left, 1.0, (0.0, 0.0));
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Left));
    }

    #[test]
    fn two_clicks_at_different_positions_is_not_double_click() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Left, 0.1, (100.0, 100.0));
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Left));
    }

    #[test]
    fn double_click_within_tolerance_is_detected() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Left, 0.1, (3.0, 4.0));
        assert!(input.is_mouse_button_double_clicked(MouseButton::Left));
    }

    #[test]
    fn double_click_is_per_button() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Right, 0.1, (0.0, 0.0));
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Left));
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Right));
    }

    #[test]
    fn double_click_cleared_after_end_frame() {
        let mut input = InputState::new();
        input.on_mouse_down(MouseButton::Left, 0.0, (0.0, 0.0));
        input.on_mouse_up(MouseButton::Left);
        input.on_mouse_down(MouseButton::Left, 0.1, (0.0, 0.0));
        assert!(input.is_mouse_button_double_clicked(MouseButton::Left));
        input.end_frame();
        assert!(!input.is_mouse_button_double_clicked(MouseButton::Left));
    }
}
