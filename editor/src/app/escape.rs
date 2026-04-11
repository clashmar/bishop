use crate::gui::modal::is_modal_open;
use engine_core::prelude::*;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EscapeOwner {
    Modal,
    Dropdown,
    Input,
    Editor,
}

thread_local! {
    static ESCAPE_OWNER: RefCell<Option<EscapeOwner>> = const { RefCell::new(None) };
}

pub(crate) fn reset_escape_resolution() {
    ESCAPE_OWNER.with(|owner| *owner.borrow_mut() = None);
}

pub(crate) fn escape_owner() -> Option<EscapeOwner> {
    ESCAPE_OWNER.with(|owner| *owner.borrow())
}

pub(crate) fn escape_available_for_editor() -> bool {
    matches!(escape_owner(), Some(EscapeOwner::Editor))
}

#[allow(dead_code)]
pub(crate) fn modal_escape_requested() -> bool {
    matches!(escape_owner(), Some(EscapeOwner::Modal))
}

pub(crate) fn resolve_escape(escape_pressed: bool) {
    reset_escape_resolution();
    if !escape_pressed {
        return;
    }

    let owner = if is_modal_open() {
        EscapeOwner::Modal
    } else if is_dropdown_open() {
        close_open_dropdowns();
        EscapeOwner::Dropdown
    } else if input_is_focused() {
        clear_all_input_focus();
        EscapeOwner::Input
    } else {
        EscapeOwner::Editor
    };

    ESCAPE_OWNER.with(|slot| *slot.borrow_mut() = Some(owner));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::modal::set_modal_open_for_test;

    #[test]
    fn resolve_escape_prioritizes_modal_dropdown_input_then_editor() {
        reset_escape_resolution();
        set_modal_open_for_test(true);
        resolve_escape(true);
        assert_eq!(escape_owner(), Some(EscapeOwner::Modal));

        reset_escape_resolution();
        set_modal_open_for_test(false);
        dropdown_state::set(
            WidgetId(1),
            dropdown_state::DropState {
                open: true,
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                scroll_offset: 0.0,
            },
        );
        update_global_dropdown_flag();
        resolve_escape(true);
        assert_eq!(escape_owner(), Some(EscapeOwner::Dropdown));

        reset_escape_resolution();
        clear_all_input_focus();
        INPUT_FOCUSED.with(|focused| *focused.borrow_mut() = true);
        resolve_escape(true);
        assert_eq!(escape_owner(), Some(EscapeOwner::Input));

        reset_escape_resolution();
        clear_all_input_focus();
        resolve_escape(true);
        assert_eq!(escape_owner(), Some(EscapeOwner::Editor));
    }

    #[test]
    fn editor_escape_is_unavailable_after_dropdown_resolution() {
        reset_escape_resolution();
        dropdown_state::set(
            WidgetId(2),
            dropdown_state::DropState {
                open: true,
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                scroll_offset: 0.0,
            },
        );
        update_global_dropdown_flag();

        resolve_escape(true);

        assert_eq!(escape_owner(), Some(EscapeOwner::Dropdown));
        assert!(!escape_available_for_editor());
    }

    #[test]
    fn modal_escape_is_visible_to_prompt_code_but_not_editor_code() {
        reset_escape_resolution();
        set_modal_open_for_test(true);

        resolve_escape(true);

        assert!(modal_escape_requested());
        assert!(!escape_available_for_editor());
    }
}
