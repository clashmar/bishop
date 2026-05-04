use crate::constants::colors;
use crate::theme::WidgetThemeMapper;
use crate::*;

/// A checkbox widget that toggles a boolean value on click.
pub struct Checkbox<'a> {
    rect: Rect,
    value: &'a mut bool,
    blocked: bool,
    visuals: WidgetVisuals,
    class_name: Option<String>,
    style_id: Option<String>,
}

impl<'a> Checkbox<'a> {
    pub fn new(rect: impl Into<Rect>, value: &'a mut bool) -> Self {
        Self {
            rect: rect.into(),
            value,
            blocked: false,
            visuals: WidgetVisuals::default(),
            class_name: None,
            style_id: None,
        }
    }

    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    pub fn visuals(mut self, visuals: WidgetVisuals) -> Self {
        self.visuals = visuals;
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class_name = Some(class.into());
        self
    }

    pub fn style_id(mut self, id: impl Into<String>) -> Self {
        self.style_id = Some(id.into());
        self
    }

    pub fn maybe_class(mut self, class: Option<&str>) -> Self {
        if let Some(c) = class {
            self.class_name = Some(c.to_string());
        }
        self
    }

    pub fn maybe_style_id(mut self, id: Option<&str>) -> Self {
        if let Some(i) = id {
            self.style_id = Some(i.to_string());
        }
        self
    }

    pub fn apply_selectors(mut self, class: Option<&str>, style_id: Option<&str>) -> Self {
        if let Some(c) = class {
            self.class_name = Some(c.to_string());
        }
        if let Some(i) = style_id {
            self.style_id = Some(i.to_string());
        }
        self
    }

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> bool {
        let class = self.class_name.as_deref();
        let id = self.style_id.as_deref();
        let theme_vs = themed_visuals_for::<Self>(class, id);
        let rect = self.rect;
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            resolve_with_theme(
                self.visuals.background,
                theme_vs.background,
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
                self.visuals.border,
                theme_vs.border,
                colors::DEFAULT_BORDER_COLOR,
            ),
        );

        if *self.value {
            let check_color =
                resolve_with_theme(self.visuals.primary, theme_vs.primary, Color::GREEN);
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

        if self.blocked || is_dropdown_open() || is_context_menu_open() {
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

impl WidgetThemeMapper for Checkbox<'_> {
    fn type_kind() -> WidgetType {
        WidgetType::Checkbox
    }
    fn theme_visuals(theme: &Theme) -> WidgetVisuals {
        WidgetVisuals {
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
