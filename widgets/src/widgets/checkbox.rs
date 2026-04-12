use crate::*;

/// Draws a checkbox widget and toggles the value on click when not blocked.
///
/// Returns true if the value was changed this frame.
pub fn gui_checkbox<C: BishopContext>(
    ctx: &mut C,
    rect: impl Into<Rect>,
    value: &mut bool,
    blocked: bool,
) -> bool {
    let rect = rect.into();
    ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, FIELD_BACKGROUND_COLOR);
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., OUTLINE_COLOR);

    if *value {
        ctx.draw_line(
            rect.x + 3.,
            rect.y + rect.h * 0.5,
            rect.x + rect.w * 0.4,
            rect.y + rect.h - 4.,
            2.,
            Color::GREEN,
        );
        ctx.draw_line(
            rect.x + rect.w * 0.4,
            rect.y + rect.h - 4.,
            rect.x + rect.w - 3.,
            rect.y + 4.,
            2.,
            Color::GREEN,
        );
    }

    if blocked || is_dropdown_open() {
        return false;
    }

    let mouse = ctx.mouse_position();
    if ctx.is_mouse_button_pressed(MouseButton::Left) && rect.contains(Vec2::new(mouse.0, mouse.1))
    {
        *value = !*value;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_support::WidgetTestContext;

    fn checkbox_rect() -> Rect {
        Rect::new(0.0, 0.0, DEFAULT_CHECKBOX_DIMS, DEFAULT_CHECKBOX_DIMS)
    }

    #[test]
    fn suppressed_checkbox_draws_but_does_not_toggle() {
        let mut value = true;
        let mut ctx = WidgetTestContext::new();
        ctx.mouse_pos = (8.0, 8.0);
        ctx.left_pressed = true;

        assert!(!gui_checkbox(&mut ctx, checkbox_rect(), &mut value, true));
        assert!(value);
        assert_eq!(ctx.rectangle_fills.len(), 1);
        assert_eq!(ctx.rectangle_lines.len(), 1);
    }

    #[test]
    fn unsuppressed_checkbox_toggles_on_click() {
        let mut value = false;
        let mut ctx = WidgetTestContext::new();
        ctx.mouse_pos = (8.0, 8.0);
        ctx.left_pressed = true;

        assert!(gui_checkbox(&mut ctx, checkbox_rect(), &mut value, false));
        assert!(value);
    }
}
