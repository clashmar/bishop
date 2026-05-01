use crate::constants::{colors, layout};
use crate::theme::WidgetThemeMapper;
use crate::*;

/// A stepper widget that allows selecting from a list of predefined values.
pub struct Stepper<'a> {
    rect: Rect,
    label: &'a str,
    steps: &'a [f32],
    current: f32,
    blocked: bool,
    visuals: WidgetVisuals,
}

impl<'a> Stepper<'a> {
    pub fn new(rect: impl Into<Rect>, label: &'a str, steps: &'a [f32], current: f32) -> Self {
        Self {
            rect: rect.into(),
            label,
            steps,
            current,
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

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> f32 {
        let theme_vs = with_theme(Self::theme_visuals);
        let rect = self.rect;
        let mut idx = self
            .steps
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (*a - self.current)
                    .abs()
                    .partial_cmp(&(*b - self.current).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        const Y_OFFSET: f32 = 15.0;

        let label = format!("{}:", self.label);
        let label_width = measure_text_ui(ctx, &label, layout::FIELD_TEXT_SIZE_16).width;

        let btn_w = layout::FIELD_TEXT_SIZE_16 * 1.2;
        let val_w = measure_text_ui(ctx, "3.0", layout::FIELD_TEXT_SIZE_16).width
            + layout::WIDGET_SPACING
            + 5.0;

        draw_text_ui(
            ctx,
            &label,
            rect.x,
            rect.y,
            layout::FIELD_TEXT_SIZE_16,
            resolve_with_theme(self.visuals.text, theme_vs.text, colors::DEFAULT_TEXT_COLOR),
        );

        let val_rect = Rect::new(
            rect.x + label_width + layout::WIDGET_SPACING,
            rect.y - Y_OFFSET,
            val_w,
            rect.h,
        );

        ctx.draw_rectangle_lines(
            val_rect.x,
            val_rect.y - 7.5,
            val_rect.w,
            btn_w + 15.0,
            2.,
            resolve_with_theme(
                self.visuals.border,
                theme_vs.border,
                colors::DEFAULT_BORDER_COLOR,
            ),
        );

        let txt = format!("{:.1}", self.steps[idx]);
        draw_text_ui(
            ctx,
            &txt,
            val_rect.x + 7.5,
            val_rect.y + 17.5,
            layout::FIELD_TEXT_SIZE_16,
            resolve_with_theme(self.visuals.text, theme_vs.text, colors::DEFAULT_TEXT_COLOR),
        );

        let decrease_rect = Rect::new(
            val_rect.x + val_w + layout::WIDGET_SPACING,
            rect.y - Y_OFFSET,
            btn_w,
            btn_w,
        );

        if Button::new(decrease_rect, "-")
            .suppressed(self.blocked)
            .visuals(self.visuals)
            .show(ctx)
            && idx > 0
        {
            idx -= 1;
        }

        let increase_rect = Rect::new(
            decrease_rect.x + btn_w + layout::WIDGET_SPACING,
            rect.y - Y_OFFSET,
            btn_w,
            btn_w,
        );
        if Button::new(increase_rect, "+")
            .suppressed(self.blocked)
            .visuals(self.visuals)
            .show(ctx)
            && idx + 1 < self.steps.len()
        {
            idx += 1;
        }

        self.steps[idx]
    }
}

impl WidgetThemeMapper for Stepper<'_> {
    fn theme_visuals(theme: &Theme) -> WidgetVisuals {
        WidgetVisuals {
            text: Some(theme.text),
            border: Some(theme.border),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_support::WidgetTestContext;

    #[test]
    fn stepper_builder_overrides_text_color() {
        let mut ctx = WidgetTestContext::new();
        let custom_visuals = WidgetVisuals {
            text: Some(Color::RED),
            ..Default::default()
        };
        let steps = [1.0, 2.0, 3.0];
        let rect = Rect::new(0.0, 0.0, 100.0, 20.0);
        let result = Stepper::new(rect, "Scale", &steps, 2.0)
            .visuals(custom_visuals)
            .show(&mut ctx);
        assert!((result - 2.0).abs() < f32::EPSILON);
        assert!(ctx.text_colors.iter().any(|c| *c == Color::RED));
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::{Theme, WidgetThemeMapper};

    #[test]
    fn stepper_theme_mapper_maps_key_roles() {
        let theme = Theme {
            text: Color::RED,
            border: Color::BLUE,
            ..Theme::default()
        };
        let visuals = Stepper::theme_visuals(&theme);
        assert_eq!(visuals.text, Some(Color::RED));
        assert_eq!(visuals.border, Some(Color::BLUE));
        assert_eq!(visuals.primary, None);
        assert_eq!(visuals.background, None);
        assert_eq!(visuals.accent, None);
        assert_eq!(visuals.hover, None);
    }
}
