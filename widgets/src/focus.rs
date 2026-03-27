use crate::*;
use std::cell::RefCell;

thread_local! {
    pub static INPUT_FOCUSED: RefCell<bool> = const { RefCell::new(false) };
}

pub fn input_is_focused() -> bool {
    INPUT_FOCUSED.with(|f| {
        let flag = f.borrow_mut();
        *flag
    })
}

thread_local! {
    static PENDING_FOCUS: RefCell<Option<WidgetId>> = const { RefCell::new(None) };
}

pub fn request_focus(id: WidgetId, is_text_input: bool) {
    clear_all_input_focus();

    INPUT_FOCUSED.with(|f| *f.borrow_mut() = true);
    PENDING_FOCUS.with(|p| *p.borrow_mut() = Some(id));

    if is_text_input {
        INPUT_TEXT_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().get_mut(&id) {
                state.focused = true;
            }
        });
    } else {
        INPUT_NUMBER_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().get_mut(&id) {
                state.focused = true;
            }
        });
    }
}

/// Consume a pending focus request for `WidgetId`.
pub fn consume_pending_focus(id: WidgetId) -> bool {
    let mut consumed = false;
    PENDING_FOCUS.with(|p| {
        let mut opt = p.borrow_mut();
        if let Some(pending_id) = *opt
            && pending_id == id
        {
            consumed = true;
            *opt = None;
        }
    });
    consumed
}

/// Helper used when the whole UI wants to lose focus.
pub fn clear_all_input_focus() {
    INPUT_FOCUSED.with(|f| *f.borrow_mut() = false);

    INPUT_TEXT_STATE.with(|s| {
        for entry in s.borrow_mut().values_mut() {
            entry.focused = false;
        }
    });

    INPUT_NUMBER_STATE.with(|s| {
        for entry in s.borrow_mut().values_mut() {
            entry.focused = false;
        }
    });

    PENDING_FOCUS.with(|p| *p.borrow_mut() = None);
}
