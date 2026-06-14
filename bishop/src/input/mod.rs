//! Input handling for keyboard and mouse.

mod gamepad;
mod keycode;
mod mouse;

use crate::types::Rect;
use glam::Vec2;

use std::cell::RefCell;

pub use gamepad::*;
pub use keycode::*;
pub use mouse::*;

thread_local! {
    static DOUBLE_CLICK_RESET_REQUESTED: RefCell<bool> = const { RefCell::new(false) };
}

/// Requests that double-click tracking be reset for the next mouse press.
/// Call this when a click has been consumed and should not count toward a
/// future double-click.
pub fn request_double_click_reset() {
    DOUBLE_CLICK_RESET_REQUESTED.with(|f| *f.borrow_mut() = true);
}

/// Returns true if a double-click reset was requested and clears the flag.
pub(crate) fn take_double_click_reset_requested() -> bool {
    DOUBLE_CLICK_RESET_REQUESTED.with(|f| {
        let was = *f.borrow();
        *f.borrow_mut() = false;
        was
    })
}

/// Input state abstraction for keyboard and mouse.
pub trait Input {
    /// Returns true if the key is currently held down.
    fn is_key_down(&self, key: KeyCode) -> bool;

    /// Returns true if the key was pressed this frame.
    fn is_key_pressed(&self, key: KeyCode) -> bool;

    /// Returns true if the key was released this frame.
    fn is_key_released(&self, key: KeyCode) -> bool;

    /// Returns true if any key was pressed this frame.
    fn any_key_pressed(&self) -> bool;

    /// Returns true if the mouse button is currently held down.
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;

    /// Returns true if the mouse button was pressed this frame.
    fn is_mouse_button_pressed(&self, button: MouseButton) -> bool;

    /// Returns true if the mouse button was released this frame.
    fn is_mouse_button_released(&self, button: MouseButton) -> bool;

    /// Returns true if the mouse button was double-clicked this frame.
    fn is_mouse_button_double_clicked(&self, button: MouseButton) -> bool;

    /// Returns the current mouse position in screen coordinates.
    fn mouse_position(&self) -> (f32, f32);

    /// Returns the mouse position delta since the last frame.
    fn mouse_delta_position(&self) -> (f32, f32);

    /// Returns the mouse wheel scroll delta (horizontal, vertical).
    fn mouse_wheel(&self) -> (f32, f32);

    /// Returns characters typed this frame for text input.
    fn chars_pressed(&self) -> Vec<char>;

    /// Returns the time in seconds since the application started.
    fn get_time(&self) -> f64;

    /// Returns the active logical-pixel clip rect, or `None` when rendering is unclipped.
    fn logical_clip_rect(&self) -> Option<Rect> {
        None
    }

    /// Returns true if the mouse is within `rect`, intersected with the active clip rect.
    fn is_mouse_over(&self, rect: Rect) -> bool {
        let (mx, my) = self.mouse_position();
        let mouse = Vec2::new(mx, my);
        match self.logical_clip_rect() {
            Some(clip) => rect.intersection(&clip).is_some_and(|r| r.contains(mouse)),
            None => rect.contains(mouse),
        }
    }
}
