use crate::constants::colors;
use crate::theme::WidgetThemeMapper;
use crate::*;

/// A checkbox widget that toggles a boolean value on click.
pub struct Checkbox<'a> {
    rect: Rect,
    value: &'a mut bool,
    blocked: bool,
    visuals: WidgetVisuals,
}

impl<'a> Checkbox<'a> {
    pub fn new(rect: impl Into<Rect>, value: &'a mut bool) -> Self {
        Self {
            rect: rect.into(),
            value,
            blocked: false,
            visuals: WidgetVisuals::default(),
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

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> bool {
        let theme_vs = with_theme(Self::theme_visuals);
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
                resolve_with_theme(self.visuals.accent, theme_vs.accent, Color::GREEN);
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
    fn theme_visuals(theme: &Theme) -> WidgetVisuals {
        WidgetVisuals {
            background: Some(theme.background),
            border: Some(theme.border),
            accent: Some(theme.accent),
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

    #[test]
    fn checkbox_builder_overrides_background_color() {
        let mut value = false;
        let mut ctx = WidgetTestContext::new();
        let rect = checkbox_rect();
        let custom_visuals = WidgetVisuals {
            background: Some(Color::RED),
            ..Default::default()
        };
        Checkbox::new(rect, &mut value)
            .visuals(custom_visuals)
            .show(&mut ctx);
        assert!(!ctx.rectangle_fills.is_empty());
        assert_eq!(ctx.rectangle_fills[0], Color::RED);
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::{Theme, WidgetThemeMapper};

    #[test]
    fn checkbox_theme_mapper_maps_key_roles() {
        let theme = Theme {
            background: Color::RED,
            border: Color::BLUE,
            accent: Color::GREEN,
            ..Theme::default()
        };
        let visuals = Checkbox::theme_visuals(&theme);
        assert_eq!(visuals.background, Some(Color::RED));
        assert_eq!(visuals.border, Some(Color::BLUE));
        assert_eq!(visuals.accent, Some(Color::GREEN));
        assert_eq!(visuals.primary, None);
        assert_eq!(visuals.text, None);
        assert_eq!(visuals.hover, None);
    }
}
