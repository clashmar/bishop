use bishop::prelude::*;

/// Tracks which button in a prompt modal is focused for keyboard navigation.
pub struct PromptFocus {
    focused_index: usize,
    button_count: usize,
}

impl PromptFocus {
    /// Creates a new focus tracker for `button_count` buttons, starting at index 0.
    pub fn new(button_count: usize) -> Self {
        Self {
            focused_index: 0,
            button_count,
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
}