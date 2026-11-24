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
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    /// Draws the widget and returns the resolved color.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Color {
        self.show_with_commit(ctx).0
    }

    /// Draws the widget and returns the resolved color and text commit state.
    pub fn show_with_commit<C: BishopContext>(self, ctx: &mut C) -> (Color, InputCommit) {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);
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
                self.base.overrides.text,
                widget_theme.text,
                colors::DEFAULT_TEXT_COLOR,
            ),
        );

        let hex = self.current.to_hex();
        let text_rect = Rect::new(text_field_x, self.rect.y, text_field_w, self.rect.h);
        let (hex_text, commit) = TextInput::new(self.id, text_rect, &hex)
            .blocked(self.base.blocked)
            .overrides(self.base.overrides)
            .max_len(6)
            .char_filter(hex_char_filter)
            .show(ctx);

        let resolved = Color::try_from_hex(&hex_text).unwrap_or(self.current);
        let commit = if hex_text == hex {
            InputCommit::Unchanged
        } else {
            commit
        };

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
            resolve_with_theme(
                self.base.overrides.border,
                widget_theme.border,
                Color::WHITE,
            ),
        );

        (resolved, commit)
    }
}

impl Widget for ColorInput {
    fn widget_type() -> WidgetType {
        WidgetType::ColorInput
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
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
mod tests {
    use super::ColorInput;
    use crate::test_utils::WidgetTestContext;
    use crate::{
        Color, InputCommit, Rect, WidgetId, reset_click_consumed, text_input_reset,
    };

    #[test]
    fn color_input_returns_text_commit_state() {
        let mut ctx = WidgetTestContext::new();
        let color = Color::new(0.2, 0.4, 0.6, 1.0);

        let (resolved, commit) = ColorInput::new(
            WidgetId::default(),
            Rect::new(0.0, 0.0, 100.0, 30.0),
            color,
        )
        .show_with_commit(&mut ctx);

        assert_eq!(resolved, color);
        assert_eq!(commit, InputCommit::Unchanged);
    }

    #[test]
    fn color_input_does_not_commit_when_focus_leaves_an_unchanged_value() {
        const FIELD: Rect = Rect::new(0.0, 0.0, 160.0, 30.0);
        let id = WidgetId(0xC010_0001);
        let color = Color::new(0.2, 0.4, 0.6, 1.0);
        reset_click_consumed();

        let mut press = WidgetTestContext::new();
        press.mouse_pos = (80.0, 15.0);
        press.left_pressed = true;
        press.left_down = true;
        ColorInput::new(id, FIELD, color).show_with_commit(&mut press);

        reset_click_consumed();
        let mut blur = WidgetTestContext::new();
        blur.mouse_pos = (200.0, 40.0);
        blur.left_pressed = true;
        let (_, commit) = ColorInput::new(id, FIELD, color).show_with_commit(&mut blur);

        assert_eq!(commit, InputCommit::Unchanged);
        text_input_reset(id);
    }
}
