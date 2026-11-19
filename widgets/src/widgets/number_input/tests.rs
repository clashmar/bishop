use super::*;
use crate::theme::Theme;

#[test]
fn number_input_theme_mapper_maps_key_roles() {
    let theme = Theme {
        background: Color::GREEN,
        accent: Color::BLUE,
        border: Color::new(0.8, 0.8, 0.8, 1.0),
        text: Color::BLACK,
        ..Theme::default()
    };
    let overrides = NumberInput::<f32>::map_theme(&theme);
    assert_eq!(overrides.background, Some(Color::GREEN));
    assert_eq!(overrides.accent, Some(Color::BLUE));
    assert_eq!(overrides.border, Some(Color::new(0.8, 0.8, 0.8, 1.0)));
    assert_eq!(overrides.text, Some(Color::BLACK));
    assert_eq!(overrides.primary, None);
    assert_eq!(overrides.surface, None);
    assert_eq!(overrides.hover, None);
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
    ) -> (String, f32, usize, Option<usize>) {
        number_input_reset(id);
        let mut state = NumberInputState::new(text.to_string());
        state.focused = true;
        state.cursor_char = cursor;
        state.selection_anchor = selection_anchor;
        INPUT_NUMBER_STATE.with(|states| {
            states.borrow_mut().insert(id, state);
        });

        let mut context = WidgetTestContext::new();
        context.keys_down.extend_from_slice(modifiers);
        context.keys_pressed.push(pressed);
        let (value, _) = NumberInput::new(id, FIELD_RECT, 12.34_f32).show(&mut context);
        let (text, cursor, selection_anchor) = INPUT_NUMBER_STATE.with(|states| {
            let states = states.borrow();
            let state = states.get(&id).expect("number input state should be retained");
            (state.text.clone(), state.cursor_char, state.selection_anchor)
        });
        number_input_reset(id);
        (text, value, cursor, selection_anchor)
    }

    #[test]
    fn number_input_modified_keys_move_by_word() {
        let text = "-12.34";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (_, _, cursor, _) = apply_key(
            WidgetId(0x4E55_1001),
            text,
            text.chars().count(),
            None,
            &[modifier],
            KeyCode::Left,
        );

        assert_eq!(cursor, prev_word_boundary(text, text.chars().count()));
    }

    #[test]
    fn number_input_modified_keys_extend_word_selection() {
        let text = "-12.34";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (_, _, cursor, anchor) = apply_key(
            WidgetId(0x4E55_1002),
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
    fn number_input_modified_keys_move_to_field_edges() {
        let text = "-12.34";
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
        let (_, _, left_cursor, _) = apply_key(
            WidgetId(0x4E55_1003),
            text,
            4,
            None,
            &left_modifiers,
            left_key,
        );
        let (_, _, right_cursor, right_anchor) = apply_key(
            WidgetId(0x4E55_1004),
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
    fn number_input_modified_keys_delete_previous_and_next_word() {
        let text = "-12.34";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (backspaced, _, backspace_cursor, _) = apply_key(
            WidgetId(0x4E55_1005),
            text,
            text.chars().count(),
            None,
            &[modifier],
            KeyCode::Backspace,
        );
        let (deleted, _, delete_cursor, _) = apply_key(
            WidgetId(0x4E55_1006),
            text,
            0,
            None,
            &[modifier],
            KeyCode::Delete,
        );

        assert_eq!(backspaced, "-12.");
        assert_eq!(backspace_cursor, 4);
        assert_eq!(deleted, ".34");
        assert_eq!(delete_cursor, 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn number_input_modified_keys_delete_to_field_edges() {
        let text = "-12.34";
        let (backspaced, _, backspace_cursor, _) = apply_key(
            WidgetId(0x4E55_1007),
            text,
            4,
            None,
            &[KeyCode::LeftSuper],
            KeyCode::Backspace,
        );
        let (deleted, _, delete_cursor, _) = apply_key(
            WidgetId(0x4E55_1008),
            text,
            2,
            None,
            &[KeyCode::LeftSuper],
            KeyCode::Delete,
        );

        assert_eq!(backspaced, "34");
        assert_eq!(backspace_cursor, 0);
        assert_eq!(deleted, "-1");
        assert_eq!(delete_cursor, 2);
    }

    #[test]
    fn number_input_modified_keys_keep_commit_range_clamping() {
        let id = WidgetId(0x4E55_1010);
        let mut state = NumberInputState::new("25".to_string());
        state.focused = true;
        state.cursor_char = 2;
        INPUT_NUMBER_STATE.with(|states| {
            states.borrow_mut().insert(id, state);
        });

        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let mut context = WidgetTestContext::new();
        context.keys_down.push(modifier);
        context.keys_pressed.extend([KeyCode::Left, KeyCode::Enter]);
        let (value, commit) = NumberInput::new(id, FIELD_RECT, 12.34_f32)
            .min(0.0)
            .max(20.0)
            .show(&mut context);

        assert_eq!(value, 20.0);
        assert_eq!(commit, InputCommit::Committed);
        number_input_reset(id);
    }

    #[test]
    fn number_input_modified_keys_delete_selection_before_word() {
        let text = "-12.34";
        let modifier = if cfg!(target_os = "macos") {
            KeyCode::LeftAlt
        } else {
            KeyCode::LeftControl
        };
        let (value, _, cursor, anchor) = apply_key(
            WidgetId(0x4E55_1009),
            text,
            3,
            Some(0),
            &[modifier],
            KeyCode::Backspace,
        );

        assert_eq!(value, ".34");
        assert_eq!(cursor, 0);
        assert_eq!(anchor, None);
    }
}
