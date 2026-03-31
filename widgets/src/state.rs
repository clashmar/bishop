use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Keys that support hold-to-repeat behavior.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepeatableKey {
    Backspace,
    Delete,
    Left,
    Right,
}

/// State for a text input widget.
pub struct TextInputState {
    pub text: String,
    pub cursor_char: usize,
    pub focused: bool,
    pub selection_anchor: Option<usize>,
    pub last_key_time: f64,
    pub repeat_key: Option<RepeatableKey>,
    pub repeat_started: bool,
    pub dragging: bool,
    pub scroll_offset_x: f32,
}

impl TextInputState {
    pub fn new(text: String) -> Self {
        let cursor_char = text.chars().count();
        Self {
            text,
            cursor_char,
            focused: false,
            selection_anchor: None,
            last_key_time: 0.0,
            repeat_key: None,
            repeat_started: false,
            dragging: false,
            scroll_offset_x: 0.0,
        }
    }
}

/// State for a number input widget.
pub struct NumberInputState {
    pub text: String,
    pub cursor_char: usize,
    pub focused: bool,
    pub selection_anchor: Option<usize>,
    pub last_key_time: f64,
    pub repeat_key: Option<RepeatableKey>,
    pub repeat_started: bool,
    pub dragging: bool,
    pub scroll_offset_x: f32,
}

impl NumberInputState {
    pub fn new(text: String) -> Self {
        let cursor_char = text.chars().count();
        Self {
            text,
            cursor_char,
            focused: false,
            selection_anchor: None,
            last_key_time: 0.0,
            repeat_key: None,
            repeat_started: false,
            dragging: false,
            scroll_offset_x: 0.0,
        }
    }
}

thread_local! {
    pub static INPUT_TEXT_STATE: RefCell<HashMap<WidgetId, TextInputState>> =
        RefCell::new(HashMap::new());
}

thread_local! {
    pub static INPUT_NUMBER_STATE: RefCell<HashMap<WidgetId, NumberInputState>> =
        RefCell::new(HashMap::new());
}

thread_local! {
    pub static DROPDOWN_OPEN: RefCell<bool> = const { RefCell::new(false) };
}

thread_local! {
    pub static CLICK_CONSUMED: RefCell<bool> = const { RefCell::new(false) };
}

/// Stable identifier for a clickable control across a mouse press/release gesture.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ClickTargetId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ArmedClickState {
    left: Option<ClickTargetId>,
    right: Option<ClickTargetId>,
}

thread_local! {
    static ARMED_CLICK_STATE: RefCell<ArmedClickState> =
        const { RefCell::new(ArmedClickState { left: None, right: None }) };
}

fn armed_click_slot(state: &mut ArmedClickState, button: MouseButton) -> Option<&mut Option<ClickTargetId>> {
    match button {
        MouseButton::Left => Some(&mut state.left),
        MouseButton::Right => Some(&mut state.right),
        _ => None,
    }
}

/// Arms a clickable control for the current mouse gesture.
pub fn arm_click_target(button: MouseButton, target: ClickTargetId) {
    ARMED_CLICK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(slot) = armed_click_slot(&mut state, button) {
            *slot = Some(target);
        }
    });
}

/// Returns true when the given control owns the current mouse gesture.
pub fn is_click_target_armed(button: MouseButton, target: ClickTargetId) -> bool {
    ARMED_CLICK_STATE.with(|state| {
        let state = state.borrow();
        match button {
            MouseButton::Left => state.left == Some(target),
            MouseButton::Right => state.right == Some(target),
            _ => false,
        }
    })
}

/// Clears the armed control for the given mouse button.
pub fn clear_click_target(button: MouseButton) {
    ARMED_CLICK_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(slot) = armed_click_slot(&mut state, button) {
            *slot = None;
        }
    });
}

/// Handles the common "press arms, release activates" interaction model.
pub fn activate_on_release(
    button: MouseButton,
    target: ClickTargetId,
    hovered: bool,
    interactive: bool,
    pressed: bool,
    released: bool,
) -> bool {
    if pressed && hovered && interactive && !is_click_consumed() {
        arm_click_target(button, target);
        consume_click();
    }

    let armed = is_click_target_armed(button, target);
    let activated = released && hovered && interactive && armed && !is_click_consumed();

    if released && armed {
        clear_click_target(button);
    }

    if activated {
        consume_click();
    }

    activated
}

pub fn is_dropdown_open() -> bool {
    DROPDOWN_OPEN.with(|f| *f.borrow())
}

/// Marks the current click as consumed, preventing other widgets from processing it.
pub fn consume_click() {
    CLICK_CONSUMED.with(|f| *f.borrow_mut() = true);
}

/// Returns true if the current click has been consumed by another widget.
pub fn is_click_consumed() -> bool {
    CLICK_CONSUMED.with(|f| *f.borrow())
}

/// Resets the click consumed flag. Call at the start of each frame.
pub fn reset_click_consumed() {
    CLICK_CONSUMED.with(|f| *f.borrow_mut() = false);
}

/// Called at the start of each frame to update widget state.
pub fn widgets_frame_start<C: BishopContext>(_ctx: &mut C) {
    tab_registry_clear();
    reset_click_consumed();
}

/// Called at the end of each frame to finalize widget state.
pub fn widgets_frame_end<C: BishopContext>(ctx: &mut C) {
    if ctx.is_mouse_button_released(MouseButton::Left) {
        clear_click_target(MouseButton::Left);
    }

    if ctx.is_mouse_button_released(MouseButton::Right) {
        clear_click_target(MouseButton::Right);
    }

    resolve_pending_tab();
    flush_dropdown_lists(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_click_state() {
        reset_click_consumed();
        clear_click_target(MouseButton::Left);
        clear_click_target(MouseButton::Right);
    }

    #[test]
    fn activate_on_release_only_fires_for_the_armed_target() {
        let first = ClickTargetId(1);
        let second = ClickTargetId(2);

        reset_click_state();
        assert!(!activate_on_release(
            MouseButton::Left,
            first,
            true,
            true,
            true,
            false,
        ));

        reset_click_consumed();
        assert!(!activate_on_release(
            MouseButton::Left,
            second,
            true,
            true,
            false,
            true,
        ));
        assert!(is_click_target_armed(MouseButton::Left, first));
    }

    #[test]
    fn activate_on_release_fires_when_press_and_release_match() {
        let target = ClickTargetId(7);

        reset_click_state();
        assert!(!activate_on_release(
            MouseButton::Left,
            target,
            true,
            true,
            true,
            false,
        ));

        reset_click_consumed();
        assert!(activate_on_release(
            MouseButton::Left,
            target,
            true,
            true,
            false,
            true,
        ));
        assert!(!is_click_target_armed(MouseButton::Left, target));
    }
}
