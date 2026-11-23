use bishop::{Text, prelude::{Rect, WgpuContext}};
use widgets::constants::{colors, layout};

pub(crate) const FIELD_HEIGHT: f32 = 30.0;
pub(crate) const FIELD_GAP: f32 = layout::WIDGET_SPACING;
pub(crate) const LABEL_WIDTH: f32 = 100.0;
pub(crate) const LABEL_Y_OFFSET: f32 = 20.0;
pub(crate) const TOP_PADDING: f32 = layout::WIDGET_SPACING;
pub(crate) const BOTTOM_GUTTER: f32 = 10.0;

pub(crate) fn draw_label(ctx: &mut WgpuContext, label: &str, rect: Rect, y: f32) {
    ctx.draw_text(
        label,
        rect.x,
        y + LABEL_Y_OFFSET,
        layout::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
}

pub(crate) fn field_rect(rect: Rect, y: f32) -> Rect {
    Rect::new(
        rect.x + LABEL_WIDTH,
        y,
        rect.w - LABEL_WIDTH,
        FIELD_HEIGHT,
    )
}
