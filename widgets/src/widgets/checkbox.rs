use crate::constants::colors;
use crate::*;

/// A checkbox widget that toggles a boolean value on click.
pub struct Checkbox<'a> {
    rect: Rect,
    value: &'a mut bool,
    base: WidgetBase,
}

impl<'a> Checkbox<'a> {
    pub fn new(rect: impl Into<Rect>, value: &'a mut bool) -> Self {
        Self {
            rect: rect.into(),
            value,
            base: WidgetBase {
                blocked: false,
                visuals: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> bool {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);
        let rect = self.rect;
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            resolve_with_theme(
                self.base.visuals.background,
                widget_theme.background,
                colors::DEFAULT_BACKGROUND_COLOR,
            ),
        );
        ctx.draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.,
            resolve_with_theme(
                self.base.visuals.border,
                widget_theme.border,
                colors::DEFAULT_BORDER_COLOR,
            ),
        );

        if *self.value {
            let check_color =
                resolve_with_theme(self.base.visuals.primary, widget_theme.primary, Color::GREEN);
            ctx.draw_line(
                rect.x + 3.,
                rect.y + rect.h * 0.5,
                rect.x + rect.w * 0.4,
                rect.y + rect.h - 4.,
                2.,
                check_color,
            );
            ctx.draw_line(
                rect.x + rect.w * 0.4,
                rect.y + rect.h - 4.,
                rect.x + rect.w - 3.,
                rect.y + 4.,
                2.,
                check_color,
            );
        }

        if self.base.blocked || is_dropdown_open() || is_context_menu_open() {
            return false;
        }

        let mouse = ctx.mouse_position();
        if ctx.is_mouse_button_pressed(MouseButton::Left)
            && rect.contains(Vec2::new(mouse.0, mouse.1))
        {
            *self.value = !*self.value;
            true
        } else {
            false
        }
    }
}

impl Widget for Checkbox<'_> {
    fn widget_type() -> WidgetType {
        WidgetType::Checkbox
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
    fn map_theme(theme: &Theme) -> WidgetTheme {
        WidgetTheme {
            background: Some(theme.background),
            border: Some(theme.border),
            primary: Some(theme.primary),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::layout;
    use crate::widgets::test_support::WidgetTestContext;

    fn checkbox_rect() -> Rect {
        Rect::new(
            0.0,
            0.0,
            layout::DEFAULT_CHECKBOX_DIMS,
            layout::DEFAULT_CHECKBOX_DIMS,
        )
    }

    #[test]
    fn suppressed_checkbox_draws_but_does_not_toggle() {
        let mut value = true;
        let mut ctx = WidgetTestContext::new();
        ctx.mouse_pos = (8.0, 8.0);
        ctx.left_pressed = true;

        assert!(!Checkbox::new(checkbox_rect(), &mut value)
            .blocked(true)
            .show(&mut ctx));
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

        assert!(Checkbox::new(checkbox_rect(), &mut value).show(&mut ctx));
        assert!(value);
    }
}
