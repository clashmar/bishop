use crate::constants::colors;
use crate::tooltip::TOOLTIP_PADDING;
use crate::*;
use bishop::text::{FontId, TextParams};

/// Horizontal text alignment for label rendering.
#[derive(Clone, Copy, Default)]
pub enum LabelAlign {
    Left,
    #[default]
    Center,
    Right,
}

enum LabelLayout {
    Rectangle(Rect),
    Point { x: f32, baseline_y: f32 },
}

/// A text label widget that renders text using theme colors.
pub struct Label {
    base: WidgetBase,
    layout: LabelLayout,
    text: String,
    font_size: f32,
    font: Option<FontId>,
    alignment: LabelAlign,
    tooltip: Option<String>,
}

impl Label {
    /// Creates a new label with the given bounds and text.
    pub fn new(rect: impl Into<Rect>, text: impl Into<String>) -> Self {
        Self::with_layout(LabelLayout::Rectangle(rect.into()), text)
    }

    /// Creates a label positioned by its text origin.
    pub fn at(text: impl Into<String>, x: f32, baseline_y: f32) -> Self {
        Self::with_layout(LabelLayout::Point { x, baseline_y }, text)
    }

    /// Sets the font size for the label text.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Selects a registered game font, or the embedded default when `None`.
    pub fn font(mut self, font: Option<FontId>) -> Self {
        self.font = font;
        self
    }

    /// Sets the horizontal text alignment.
    pub fn alignment(mut self, align: LabelAlign) -> Self {
        self.alignment = align;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.base.overrides.text = Some(color.into());
        self
    }

    /// Sets optional tooltip text for the label.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Draws the label.
    pub fn show<C: BishopContext>(self, ctx: &mut C) {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);

        let text_color = resolve_with_theme(
            self.base.overrides.text,
            widget_theme.text,
            colors::DEFAULT_TEXT_COLOR,
        );
        let logical_font_size = self.font_size.max(1.0);
        let base_font_size = logical_font_size
            .round()
            .clamp(1.0, u16::MAX as f32);
        let params = TextParams {
            font: self.font,
            font_size: base_font_size as u16,
            font_scale: logical_font_size / base_font_size,
            ..TextParams::default()
        };
        let text_dims = ctx.measure_text_ex(&self.text, &params);
        let (text_x, text_y, tooltip_bounds) = match self.layout {
            LabelLayout::Rectangle(rect) => {
                let text_x = match self.alignment {
                    LabelAlign::Left => rect.x,
                    LabelAlign::Center => rect.x + (rect.w - text_dims.width) / 2.0,
                    LabelAlign::Right => rect.x + rect.w - text_dims.width,
                };
                let text_y = rect.y + (rect.h - text_dims.height) / 2.0 + text_dims.offset_y;
                (text_x, text_y, rect)
            }
            LabelLayout::Point { x, baseline_y } => {
                let tooltip_bounds = Rect::new(
                    x - TOOLTIP_PADDING,
                    baseline_y - text_dims.offset_y - TOOLTIP_PADDING,
                    text_dims.width + TOOLTIP_PADDING * 2.0,
                    text_dims.height + TOOLTIP_PADDING * 2.0,
                );
                (x, baseline_y, tooltip_bounds)
            }
        };

        ctx.draw_text_ex(
            &self.text,
            text_x,
            text_y,
            TextParams {
                color: text_color,
                ..params
            },
        );
        if let Some(text) = self.tooltip {
            offer_tooltip(tooltip_bounds, text);
        }
    }

    fn with_layout(layout: LabelLayout, text: impl Into<String>) -> Self {
        Self {
            base: WidgetBase {
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
            layout,
            text: text.into(),
            font_size: 20.0,
            font: None,
            alignment: LabelAlign::default(),
            tooltip: None,
        }
    }
}

impl Widget for Label {
    fn widget_type() -> WidgetType {
        WidgetType::Label
    }

    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::WidgetTestContext;

    #[test]
    fn label_rectangle_tooltip_shows_after_hover_delay() {
        let bounds = Rect::new(10.0, 10.0, 120.0, 30.0);
        let tooltip = "Label tooltip";
        let mut ctx = WidgetTestContext::new();
        ctx.mouse_pos = (20.0, 20.0);

        widgets_frame_start(&mut ctx);
        Label::new(bounds, "Label").tooltip(tooltip).show(&mut ctx);
        widgets_frame_end(&mut ctx);
        assert!(!ctx.draw_text_calls.iter().any(|call| call.text == tooltip));

        ctx.set_time(0.5);
        widgets_frame_start(&mut ctx);
        Label::new(bounds, "Label").tooltip(tooltip).show(&mut ctx);
        widgets_frame_end(&mut ctx);
        assert!(ctx.draw_text_calls.iter().any(|call| call.text == tooltip));
    }

    #[test]
    fn label_point_tooltip_uses_measured_text_bounds() {
        let tooltip = "Point label tooltip";
        let mut ctx = WidgetTestContext::new();
        ctx.text_dims = Some(TextDimensions {
            width: 42.0,
            height: 16.0,
            offset_y: 12.0,
        });
        ctx.mouse_pos = (30.0, 52.0);

        widgets_frame_start(&mut ctx);
        Label::at("Overlay", 24.0, 60.0)
            .font_size(16.0)
            .text_color(Color::WHITE)
            .tooltip(tooltip)
            .show(&mut ctx);
        widgets_frame_end(&mut ctx);
        assert!(!ctx.draw_text_calls.iter().any(|call| call.text == tooltip));

        ctx.set_time(0.5);
        widgets_frame_start(&mut ctx);
        Label::at("Overlay", 24.0, 60.0)
            .font_size(16.0)
            .text_color(Color::WHITE)
            .tooltip(tooltip)
            .show(&mut ctx);
        widgets_frame_end(&mut ctx);
        assert!(ctx.draw_text_calls.iter().any(|call| call.text == tooltip));
    }

    #[test]
    fn selected_font_is_used_for_label_measurement_and_drawing() {
        use bishop::text::FontId;

        let font_id = FontId(9);
        let mut ctx = WidgetTestContext::new();
        Label::new(Rect::new(0.0, 0.0, 120.0, 30.0), "Label")
            .font(Some(font_id))
            .font_size(20.5)
            .show(&mut ctx);

        assert_eq!(ctx.measured_font.get(), Some(font_id));
        assert!((ctx.measured_font_scale.get() * 21.0 - 20.5).abs() < f32::EPSILON);
        assert_eq!(ctx.draw_text_calls[0].font, Some(font_id));
        assert_eq!(ctx.draw_text_calls[0].font_scale, ctx.measured_font_scale.get());
    }

    #[test]
    fn label_without_tooltip_does_not_draw_tooltip_text() {
        let mut ctx = WidgetTestContext::new();
        ctx.mouse_pos = (20.0, 20.0);
        ctx.set_time(1.0);

        widgets_frame_start(&mut ctx);
        Label::new(Rect::new(10.0, 10.0, 120.0, 30.0), "Label").show(&mut ctx);
        widgets_frame_end(&mut ctx);

        assert!(!ctx
            .draw_text_calls
            .iter()
            .any(|call| call.text == "Tooltip text"));
    }
}
