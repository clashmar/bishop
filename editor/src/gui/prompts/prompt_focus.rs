use bishop::prelude::*;

/// Tracks which button in a prompt modal is focused for keyboard navigation.
pub struct PromptFocus {
    focused_index: usize,
    button_count: usize,
    last_hovered_index: Option<usize>,
}

impl PromptFocus {
    /// Creates a new focus tracker for `button_count` buttons, starting at index 0.
    /// The last_hovered_index starts as None and will be set when mouse hover is detected.
    pub fn new(button_count: usize) -> Self {
        Self {
            focused_index: 0,
            button_count,
            last_hovered_index: None,
        }
    }

    /// Reads arrow and Tab key input to move focus.
    /// Call once per frame before drawing buttons.
    pub fn navigate<C: Input>(&mut self, ctx: &C) {
        if self.button_count <= 1 {
            return;
        }

        if ctx.is_key_pressed(KeyCode::Right)
            || (ctx.is_key_pressed(KeyCode::Tab)
                && !ctx.is_key_down(KeyCode::LeftShift)
                && !ctx.is_key_down(KeyCode::RightShift))
        {
            self.focused_index = (self.focused_index + 1) % self.button_count;
        }
        if ctx.is_key_pressed(KeyCode::Left)
            || (ctx.is_key_pressed(KeyCode::Tab)
                && (ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift)))
        {
            if self.focused_index == 0 {
                self.focused_index = self.button_count - 1;
            } else {
                self.focused_index -= 1;
            }
        }
    }

    /// Reads arrow/Tab keys and mouse hover to move focus.
    /// Call once per frame before drawing buttons.
    /// Only changes focus when mouse enters a new button, not on stay/leave.
    pub fn navigate_with_mouse<C: Input>(&mut self, ctx: &C, button_rects: &[Rect]) {
        self.navigate(ctx);

        let mouse: Vec2 = ctx.mouse_position().into();
        let hovered = button_rects.iter().position(|r| r.contains(mouse));

        if let Some(i) = hovered {
            if self.last_hovered_index != Some(i) {
                self.focused_index = i;
            }
        }
        self.last_hovered_index = hovered;
    }

    /// Returns true if button at `index` should render as focused.
    pub fn is_focused(&self, index: usize) -> bool {
        self.focused_index == index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockInput;

    #[test]
    fn new_starts_at_index_zero() {
        let focus = PromptFocus::new(3);
        assert!(focus.is_focused(0));
        assert!(!focus.is_focused(1));
        assert!(!focus.is_focused(2));
    }

    #[test]
    fn right_arrow_increments() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Right);
        focus.navigate(&ctx);
        assert!(focus.is_focused(1));
    }

    #[test]
    fn right_arrow_wraps() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Right);
        focus.navigate(&ctx); // 0 -> 1
        let mut ctx2 = MockInput::new();
        ctx2.press(KeyCode::Right);
        focus.navigate(&ctx2); // 1 -> 2
        let mut ctx3 = MockInput::new();
        ctx3.press(KeyCode::Right);
        focus.navigate(&ctx3); // 2 -> 0 (wrap)
        assert!(focus.is_focused(0));
    }

    #[test]
    fn left_arrow_decrements_and_wraps() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Left);
        focus.navigate(&ctx); // 0 -> 2 (wrap)
        assert!(focus.is_focused(2));
        let mut ctx2 = MockInput::new();
        ctx2.press(KeyCode::Left);
        focus.navigate(&ctx2); // 2 -> 1
        assert!(focus.is_focused(1));
    }

    #[test]
    fn tab_increments_like_right() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Tab);
        focus.navigate(&ctx);
        assert!(focus.is_focused(1));
    }

    #[test]
    fn shift_tab_decrements_like_left() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Left);
        focus.navigate(&ctx); // 0 -> 2
        let mut ctx2 = MockInput::new();
        ctx2.press(KeyCode::Tab);
        ctx2.hold(KeyCode::LeftShift);
        focus.navigate(&ctx2); // 2 -> 1
        assert!(focus.is_focused(1));
    }

    #[test]
    fn single_button_is_noop() {
        let mut focus = PromptFocus::new(1);
        let mut ctx = MockInput::new();
        ctx.press(KeyCode::Right);
        focus.navigate(&ctx);
        assert!(focus.is_focused(0));
    }

    #[test]
    fn hover_enters_sets_focus() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        let rects = [
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(110.0, 0.0, 100.0, 40.0),
            Rect::new(220.0, 0.0, 100.0, 40.0),
        ];

        // Mouse over button 1 (center of second rect)
        ctx.set_mouse_position((160.0, 20.0));
        focus.navigate_with_mouse(&ctx, &rects);
        assert!(focus.is_focused(1));
        assert!(!focus.is_focused(0));
    }

    #[test]
    fn hover_stays_no_change_when_same_element() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        let rects = [
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(110.0, 0.0, 100.0, 40.0),
            Rect::new(220.0, 0.0, 100.0, 40.0),
        ];

        // Enter button 1
        ctx.set_mouse_position((160.0, 20.0));
        focus.navigate_with_mouse(&ctx, &rects);
        assert!(focus.is_focused(1));

        // Stay on button 1 — same mouse position
        let mut ctx2 = MockInput::new();
        ctx2.set_mouse_position((160.0, 20.0));
        focus.navigate_with_mouse(&ctx2, &rects);
        assert!(focus.is_focused(1));
    }

    #[test]
    fn hover_leaves_focus_stays() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        let rects = [
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(110.0, 0.0, 100.0, 40.0),
            Rect::new(220.0, 0.0, 100.0, 40.0),
        ];

        // Enter button 1
        ctx.set_mouse_position((160.0, 20.0));
        focus.navigate_with_mouse(&ctx, &rects);
        assert!(focus.is_focused(1));

        // Mouse leaves all rects
        let mut ctx2 = MockInput::new();
        ctx2.set_mouse_position((-10.0, -10.0));
        focus.navigate_with_mouse(&ctx2, &rects);
        // Focus should stay on button 1 (no snap back)
        assert!(focus.is_focused(1));
    }

    #[test]
    fn keyboard_nav_while_hovered_let_keyboard_win() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        let rects = [
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(110.0, 0.0, 100.0, 40.0),
            Rect::new(220.0, 0.0, 100.0, 40.0),
        ];

        // Hover button 0 first
        ctx.set_mouse_position((50.0, 20.0));
        focus.navigate_with_mouse(&ctx, &rects);
        assert!(focus.is_focused(0));

        // Press Right arrow while mouse stays on button 0
        let mut ctx2 = MockInput::new();
        ctx2.set_mouse_position((50.0, 20.0));
        ctx2.press(KeyCode::Right);
        focus.navigate_with_mouse(&ctx2, &rects);
        // Keyboard should move focus to 1 (last_hovered_index is still Some(0), so hover doesn't change)
        assert!(focus.is_focused(1));

        // Next frame: no keyboard, mouse still on button 0 — no change (last_hovered still Some(0))
        let mut ctx3 = MockInput::new();
        ctx3.set_mouse_position((50.0, 20.0));
        focus.navigate_with_mouse(&ctx3, &rects);
        assert!(focus.is_focused(1));
    }

    #[test]
    fn keyboard_then_new_hover_shifts_focus() {
        let mut focus = PromptFocus::new(3);
        let mut ctx = MockInput::new();
        let rects = [
            Rect::new(0.0, 0.0, 100.0, 40.0),
            Rect::new(110.0, 0.0, 100.0, 40.0),
            Rect::new(220.0, 0.0, 100.0, 40.0),
        ];

        // Hover button 0
        ctx.set_mouse_position((50.0, 20.0));
        focus.navigate_with_mouse(&ctx, &rects);
        assert!(focus.is_focused(0));

        // Press Right to go to button 1 (mouse still on button 0)
        let mut ctx2 = MockInput::new();
        ctx2.set_mouse_position((50.0, 20.0));
        ctx2.press(KeyCode::Right);
        focus.navigate_with_mouse(&ctx2, &rects);
        assert!(focus.is_focused(1));

        // Now mouse moves to button 2 — hover enters new element
        let mut ctx3 = MockInput::new();
        ctx3.set_mouse_position((270.0, 20.0));
        focus.navigate_with_mouse(&ctx3, &rects);
        assert!(focus.is_focused(2));
    }
}
