use crate::*;

/// Draws a checkbox widget and toggles the value on click.
///
/// Returns true if the value was changed this frame.
pub fn gui_checkbox<C: BishopContext>(
    ctx: &mut C,
    rect: impl Into<Rect>,
    value: &mut bool,
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

    if is_dropdown_open() {
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
