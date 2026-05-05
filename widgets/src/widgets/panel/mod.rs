use crate::constants::colors;
use crate::*;

/// A decorative panel widget that renders a filled rectangle using theme colors.
pub struct Panel {
    base: WidgetBase,
    rect: Rect,
}

impl Panel {
    /// Creates a new panel filling the given rect.
    pub fn new(rect: impl Into<Rect>) -> Self {
        Self {
            rect: rect.into(),
            base: WidgetBase {
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    /// Sets the panel's bounding rect.
    pub fn rect(mut self, rect: impl Into<Rect>) -> Self {
        self.rect = rect.into();
        self
    }

    /// Draws the panel.
    pub fn show<C: BishopContext>(self, ctx: &mut C) {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);

        let bg = resolve_with_theme(
            self.base.overrides.panel,
            widget_theme.panel,
            colors::DEFAULT_PANEL_COLOR,
        );

        ctx.draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg);

        let border_color = resolve_with_theme(
            self.base.overrides.border,
            widget_theme.border,
            colors::DEFAULT_BORDER_COLOR,
        );
        ctx.draw_rectangle_lines(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            1.0,
            border_color,
        );
    }
}

impl Widget for Panel {
    fn widget_type() -> WidgetType {
        WidgetType::Panel
    }

    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}
