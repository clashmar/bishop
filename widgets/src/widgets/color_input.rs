use crate::*;

/// A hex color input widget with a color swatch preview.
pub struct ColorInput {
    id: WidgetId,
    rect: Rect,
    current: Color,
    blocked: bool,
}

impl ColorInput {
    /// Creates a new color input widget with the given id, rect, and current color.
    pub fn new(id: WidgetId, rect: impl Into<Rect>, current: Color) -> Self {
        Self {
            id,
            rect: rect.into(),
            current,
            blocked: false,
        }
    }

    /// Sets whether the input is blocked from interaction.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Draws the widget and returns the resolved color.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Color {
        let swatch_size = self.rect.h;
        let gap = 4.0;
        let prefix_width = measure_text_ui(ctx, "#", DEFAULT_FONT_SIZE_16).width + 2.0;
        let text_field_x = self.rect.x + swatch_size + gap + prefix_width;
        let text_field_w = self.rect.w - swatch_size - gap - prefix_width;

        let prefix_x = self.rect.x + swatch_size + gap;
        let prefix_y = self.rect.y + self.rect.h * 0.7;
        draw_text_ui(
            ctx,
            "#",
            prefix_x,
            prefix_y,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );

        let hex = self.current.to_hex();
        let text_rect = Rect::new(text_field_x, self.rect.y, text_field_w, self.rect.h);
        let (hex_text, _focused) = TextInput::new(self.id, text_rect, &hex)
            .blocked(self.blocked)
            .max_len(6)
            .char_filter(hex_char_filter)
            .show(ctx);

        let resolved = Color::from_hex(&hex_text).unwrap_or(self.current);

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
            Color::WHITE,
        );

        resolved
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
