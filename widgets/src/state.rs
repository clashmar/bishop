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
    resolve_pending_tab();
    flush_dropdown_lists(ctx);
}
