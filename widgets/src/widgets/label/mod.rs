use crate::constants::colors;
use crate::*;

/// Horizontal text alignment for label rendering.
#[derive(Clone, Copy, Default)]
pub enum LabelAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// A text label widget that renders text using theme colors.
pub struct Label {
    base: WidgetBase,
    rect: Rect,
    text: String,
    font_size: f32,
    alignment: LabelAlign,
}

impl Label {
    /// Creates a new label with the given bounds and text.
    pub fn new(rect: impl Into<Rect>, text: impl Into<String>) -> Self {
        Self {
            rect: rect.into(),
            text: text.into(),
            font_size: 20.0,
            alignment: LabelAlign::default(),
            base: WidgetBase {
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    /// Sets the font size for the label text.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the horizontal text alignment.
    pub fn alignment(mut self, align: LabelAlign) -> Self {
        self.alignment = align;
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

        let txt_dims = ctx.measure_text(&self.text, self.font_size);
        let txt_x = match self.alignment {
            LabelAlign::Left => self.rect.x,
            LabelAlign::Center => self.rect.x + (self.rect.w - txt_dims.width) / 2.0,
            LabelAlign::Right => self.rect.x + self.rect.w - txt_dims.width,
        };
        let txt_y = self.rect.y + (self.rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;

        ctx.draw_text(&self.text, txt_x, txt_y, self.font_size, text_color);
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
