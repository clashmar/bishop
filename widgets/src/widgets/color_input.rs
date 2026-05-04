use crate::constants::{colors, layout};
use crate::*;

/// A hex color input widget with a color swatch preview.
pub struct ColorInput {
    id: WidgetId,
    rect: Rect,
    current: Color,
    base: WidgetBase,
}

impl ColorInput {
    /// Creates a new color input widget with the given id, rect, and current color.
    pub fn new(id: WidgetId, rect: impl Into<Rect>, current: Color) -> Self {
        Self {
            id,
            rect: rect.into(),
            current,
            base: WidgetBase {
                blocked: false,
                visuals: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    /// Draws the widget and returns the resolved color.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Color {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let theme_vs = resolve_theme_for::<Self>(class, id);
        let swatch_size = self.rect.h;
        let gap = 4.0;
        let prefix_width = measure_text_ui(ctx, "#", layout::DEFAULT_FONT_SIZE_16).width + 2.0;
        let text_field_x = self.rect.x + swatch_size + gap + prefix_width;
        let text_field_w = self.rect.w - swatch_size - gap - prefix_width;

        let prefix_x = self.rect.x + swatch_size + gap;
        let prefix_y = self.rect.y + self.rect.h * 0.7;
        draw_text_ui(
            ctx,
            "#",
            prefix_x,
            prefix_y,
            layout::DEFAULT_FONT_SIZE_16,
            resolve_with_theme(
                self.base.visuals.text,
                theme_vs.text,
                colors::DEFAULT_TEXT_COLOR,
            ),
        );

        let hex = self.current.to_hex();
        let text_rect = Rect::new(text_field_x, self.rect.y, text_field_w, self.rect.h);
        let (hex_text, _focused) = TextInput::new(self.id, text_rect, &hex)
            .blocked(self.base.blocked)
            .visuals(self.base.visuals)
            .max_len(6)
            .char_filter(hex_char_filter)
            .show(ctx);

        let resolved = Color::try_from_hex(&hex_text).unwrap_or(self.current);

        let swatch_rect = Rect::new(self.rect.x, self.rect.y, swatch_size, swatch_size);
        ctx.draw_rectangle(
            swatch_rect.x,
            swatch_rect.y,
            swatch_rect.w,
            swatch_rect.h,
            resolved,
        );
        ctx.draw_rectangle_lines(
            swatch_rect.x,
            swatch_rect.y,
            swatch_rect.w,
            swatch_rect.h,
            2.0,
            resolve_with_theme(self.base.visuals.border, theme_vs.border, Color::WHITE),
        );

        resolved
    }
}

impl Widget for ColorInput {
    fn widget_type() -> WidgetType {
        WidgetType::ColorInput
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
    fn map_theme(theme: &Theme) -> WidgetTheme {
        WidgetTheme {
            background: Some(theme.surface),
            border: Some(theme.border),
            accent: Some(theme.accent),
            text: Some(theme.text),
            ..Default::default()
        }
    }
}

/// Resets the color input state for the given widget id.
pub fn color_input_reset(id: WidgetId) {
    text_input_reset(id);
}

fn hex_char_filter(c: char) -> Option<char> {
    if c.is_ascii_hexdigit() {
        Some(c.to_ascii_uppercase())
    } else {
        None
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn color_input_theme_mapper_maps_key_roles() {
        let theme = Theme {
            surface: Color::GREEN,
            border: Color::BLUE,
            accent: Color::RED,
            text: Color::BLACK,
            ..Theme::default()
        };
        let visuals = ColorInput::map_theme(&theme);
        assert_eq!(visuals.background, Some(Color::GREEN));
        assert_eq!(visuals.border, Some(Color::BLUE));
        assert_eq!(visuals.accent, Some(Color::RED));
        assert_eq!(visuals.text, Some(Color::BLACK));
        assert_eq!(visuals.primary, None);
        assert_eq!(visuals.hover, None);
    }
}
