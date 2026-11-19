use super::*;
use crate::theme::Theme;

#[test]
fn text_input_theme_mapper_maps_key_roles() {
    let theme = Theme {
        background: Color::GREEN,
        accent: Color::BLUE,
        border: Color::new(0.8, 0.8, 0.8, 1.0),
        text: Color::BLACK,
        ..Theme::default()
    };
    let overrides = TextInput::map_theme(&theme);
    assert_eq!(overrides.background, Some(Color::GREEN));
    assert_eq!(overrides.accent, Some(Color::BLUE));
    assert_eq!(overrides.border, Some(Color::new(0.8, 0.8, 0.8, 1.0)));
    assert_eq!(overrides.text, Some(Color::BLACK));
    assert_eq!(overrides.primary, None);
    assert_eq!(overrides.surface, None);
    assert_eq!(overrides.hover, None);
}

#[cfg(feature = "test-utils")]
mod clear_button_tests {
    use super::*;
    use crate::test_utils::WidgetTestContext;

    const FIELD_RECT: Rect = Rect::new(0.0, 0.0, 160.0, 30.0);
    const INITIAL_TEXT: &str = "a very long value that requires horizontal scrolling";

    #[test]
    fn text_input_clear_button_is_opt_in() {
        reset_click_consumed();
        let id = WidgetId(0xC1EA_0001);
        let mut press = WidgetTestContext::new();
        press.mouse_pos = (145.0, 15.0);
        press.left_pressed = true;
        press.left_down = true;
        TextInput::new(id, FIELD_RECT, "value").show(&mut press);

        reset_click_consumed();
        let mut release = WidgetTestContext::new();
        release.mouse_pos = (145.0, 15.0);
        release.left_released = true;
        let (value, _) = TextInput::new(id, FIELD_RECT, "value").show(&mut release);

        assert_eq!(value, "value");
        text_input_reset(id);
    }

    #[test]
    fn text_input_clear_button_does_not_draw_hover_fill() {
        let id = WidgetId(0xC1EA_0005);
        let mut context = WidgetTestContext::new();
        context.mouse_pos = (145.0, 15.0);

        TextInput::new(id, FIELD_RECT, "value")
            .clear_button()
            .show(&mut context);

        assert_eq!(context.rect_calls.len(), 2);
        assert_eq!(
            context.rectangle_fills[1],
            Color::new(0.0, 0.0, 0.0, 0.0)
        );
        text_input_reset(id);
    }

    #[test]
    fn text_input_clear_button_clears_text_and_edit_state() {
        reset_click_consumed();
        let id = WidgetId(0xC1EA_0002);
        let mut initial_state = TextInputState::new(INITIAL_TEXT.to_string());
        initial_state.focused = true;
        initial_state.selection_anchor = Some(2);
        initial_state.scroll_offset_x = 30.0;
        INPUT_TEXT_STATE.with(|state| {
            state.borrow_mut().insert(id, initial_state);
        });
        let mut press = WidgetTestContext::new();
        press.mouse_pos = (145.0, 15.0);
        press.left_pressed = true;
        press.left_down = true;
        TextInput::new(id, FIELD_RECT, INITIAL_TEXT)
            .clear_button()
            .show(&mut press);

        reset_click_consumed();
        let mut release = WidgetTestContext::new();
        release.mouse_pos = (145.0, 15.0);
        release.left_released = true;
        let (value, commit) =
            TextInput::new(id, FIELD_RECT, INITIAL_TEXT)
                .clear_button()
                .show(&mut release);

        assert_eq!(value, "");
        assert_eq!(commit, InputCommit::Committed);
        INPUT_TEXT_STATE.with(|state| {
            let states = state.borrow();
            let input = states.get(&id).expect("text input state should be retained");
            assert_eq!(input.cursor_char, 0);
            assert_eq!(input.selection_anchor, None);
            assert_eq!(input.scroll_offset_x, 0.0);
        });
        text_input_reset(id);
    }

    #[test]
    fn text_input_clear_button_is_inactive_for_empty_text() {
        reset_click_consumed();
        let id = WidgetId(0xC1EA_0003);
        let mut press = WidgetTestContext::new();
        press.mouse_pos = (145.0, 15.0);
        press.left_pressed = true;
        press.left_down = true;
        TextInput::new(id, FIELD_RECT, "").clear_button().show(&mut press);

        reset_click_consumed();
        let mut release = WidgetTestContext::new();
        release.mouse_pos = (145.0, 15.0);
        release.left_released = true;
        let (_, commit) = TextInput::new(id, FIELD_RECT, "").clear_button().show(&mut release);

        assert_ne!(commit, InputCommit::Committed);
        text_input_reset(id);
    }

    #[test]
    fn blocked_text_input_clear_button_does_not_clear_text() {
        reset_click_consumed();
        let id = WidgetId(0xC1EA_0004);
        let mut press = WidgetTestContext::new();
        press.mouse_pos = (145.0, 15.0);
        press.left_pressed = true;
        press.left_down = true;
        TextInput::new(id, FIELD_RECT, "value")
            .clear_button()
            .blocked(true)
            .show(&mut press);

        reset_click_consumed();
        let mut release = WidgetTestContext::new();
        release.mouse_pos = (145.0, 15.0);
        release.left_released = true;
        let (value, _) = TextInput::new(id, FIELD_RECT, "value")
            .clear_button()
            .blocked(true)
            .show(&mut release);

        assert_eq!(value, "value");
        text_input_reset(id);
    }
}

#[cfg(feature = "test-utils")]
mod modified_key_tests {
    use super::*;
    use crate::test_utils::WidgetTestContext;

    const FIELD_RECT: Rect = Rect::new(0.0, 0.0, 160.0, 30.0);

    fn apply_key(
        id: WidgetId,
        text: &str,
        cursor: usize,
        selection_anchor: Option<usize>,
        modifiers: &[KeyCode],
        pressed: KeyCode,
    ) -> (String, usize, Option<usize>) {
        text_input_reset(id);
        let mut state = TextInputState::new(text.to_string());
        state.focused = true;
        state.cursor_char = cursor;
        state.selection_anchor = selection_anchor;
        INPUT_TEXT_STATE.with(|states| {
            states.borrow_mut().insert(id, state);
        });

        let mut context = WidgetTestContext::new();
        context.keys_down.extend_from_slice(modifiers);
        context.keys_pressed.push(pressed);
        let (value, _) = TextInput::new(id, FIELD_RECT, text).show(&mut context);
        let (cursor, selection_anchor) = INPUT_TEXT_STATE.with(|states| {
            let states = states.borrow();
            let state = states.get(&id).expect("text input state should be retained");
            (state.cursor_char, state.selection_anchor)
        });
        text_input_reset(id);
        (value, cursor, selection_anchor)
    }

    #[test]
    fn text_input_modified_keys_move_by_word() {
        let text = "one two";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (_, cursor, _) = apply_key(
            WidgetId(0xC1EA_1001),
            text,
            text.chars().count(),
            None,
            &[modifier],
            KeyCode::Left,
        );

        assert_eq!(cursor, prev_word_boundary(text, text.chars().count()));
    }

    #[test]
    fn text_input_modified_keys_extend_word_selection() {
        let text = "one two";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (_, cursor, anchor) = apply_key(
            WidgetId(0xC1EA_1002),
            text,
            0,
            None,
            &[modifier, KeyCode::LeftShift],
            KeyCode::Right,
        );

        assert_eq!(cursor, next_word_boundary(text, 0));
        assert_eq!(anchor, Some(0));
    }

    #[test]
    fn text_input_modified_keys_move_to_line_edges() {
        let text = "one two";
        let (left_modifiers, left_key, right_modifiers, right_key) = if cfg!(target_os = "macos") {
            (
                vec![KeyCode::LeftSuper],
                KeyCode::Left,
                vec![KeyCode::LeftSuper, KeyCode::LeftShift],
                KeyCode::Right,
            )
        } else {
            (
                vec![KeyCode::LeftControl],
                KeyCode::Home,
                vec![KeyCode::LeftControl, KeyCode::LeftShift],
                KeyCode::End,
            )
        };
        let (_, left_cursor, _) = apply_key(
            WidgetId(0xC1EA_1003),
            text,
            4,
            None,
            &left_modifiers,
            left_key,
        );
        let (_, right_cursor, right_anchor) = apply_key(
            WidgetId(0xC1EA_1004),
            text,
            0,
            None,
            &right_modifiers,
            right_key,
        );

        assert_eq!(left_cursor, 0);
        assert_eq!(right_cursor, text.chars().count());
        assert_eq!(right_anchor, Some(0));
    }

    #[test]
    fn text_input_modified_keys_delete_previous_and_next_word() {
        let text = "one two";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (backspaced, backspace_cursor, _) = apply_key(
            WidgetId(0xC1EA_1005),
            text,
            text.chars().count(),
            None,
            &[modifier],
            KeyCode::Backspace,
        );
        let (deleted, delete_cursor, _) = apply_key(
            WidgetId(0xC1EA_1006),
            text,
            4,
            None,
            &[modifier],
            KeyCode::Delete,
        );

        assert_eq!(backspaced, "one ");
        assert_eq!(backspace_cursor, 4);
        assert_eq!(deleted, "one ");
        assert_eq!(delete_cursor, 4);
    }

    #[test]
    fn text_input_modified_keys_delete_selection_before_word() {
        let text = "one two";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (value, cursor, anchor) = apply_key(
            WidgetId(0xC1EA_1007),
            text,
            3,
            Some(0),
            &[modifier],
            KeyCode::Backspace,
        );

        assert_eq!(value, " two");
        assert_eq!(cursor, 0);
        assert_eq!(anchor, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn text_input_modified_keys_delete_to_line_edges() {
        let text = "one two";
        let (backspaced, backspace_cursor, _) = apply_key(
            WidgetId(0xC1EA_1008),
            text,
            4,
            None,
            &[KeyCode::LeftSuper],
            KeyCode::Backspace,
        );
        let (deleted, delete_cursor, _) = apply_key(
            WidgetId(0xC1EA_1009),
            text,
            4,
            None,
            &[KeyCode::LeftSuper],
            KeyCode::Delete,
        );

        assert_eq!(backspaced, "two");
        assert_eq!(backspace_cursor, 0);
        assert_eq!(deleted, "one ");
        assert_eq!(delete_cursor, 4);
    }

    #[test]
    fn text_input_modified_keys_preserve_unicode_cursor_indices() {
        let text = "éclair 猫犬";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (_, cursor, _) = apply_key(
            WidgetId(0xC1EA_1010),
            text,
            text.chars().count(),
            None,
            &[modifier],
            KeyCode::Left,
        );

        assert_eq!(cursor, prev_word_boundary(text, text.chars().count()));
    }
}
