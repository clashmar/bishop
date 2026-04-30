// editor/src/gui/prompts/helpers.rs
use crate::gui::prompts::constants::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;

/// Returns a centered content rect inside a modal for prompt widgets.
pub fn prompt_content_rect(modal_rect: Rect, content_h: f32) -> Rect {
    let inner_w = modal_rect.w * PROMPT_CONTENT_WIDTH_RATIO;
    let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;
    let inner_y = modal_rect.y + (modal_rect.h - content_h) / 2.0;
    Rect::new(inner_x, inner_y, inner_w, content_h)
}

/// Draws prompt label text using top-aligned layout coordinates.
pub fn draw_prompt_label<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    x: f32,
    top_y: f32,
) -> TextDimensions {
    let dims = measure_text(ctx, text, layout::DEFAULT_FONT_SIZE_16);
    ctx.draw_text(
        text,
        x,
        top_y + dims.offset_y,
        layout::DEFAULT_FONT_SIZE_16,
        Color::WHITE,
    );
    dims
}

/// Supplies centered rects for confirm/cancel buttons.
pub fn confirm_cancel_rects(rect: Rect, btn_y: f32) -> (Rect, Rect) {
    let spacing = (rect.w - 2.0 * BUTTON_W) / 3.0;
    let confirm_rect = Rect::new(rect.x + spacing, btn_y, BUTTON_W, BUTTON_H);
    let cancel_rect = Rect::new(rect.x + 2.0 * spacing + BUTTON_W, btn_y, BUTTON_W, BUTTON_H);
    (confirm_rect, cancel_rect)
}

/// Supplies centered rects for a three-button inline action row.
pub fn three_button_rects(rect: Rect, btn_y: f32) -> (Rect, Rect, Rect) {
    let spacing = (rect.w - 3.0 * BUTTON_W) / 2.0;
    let confirm_rect = Rect::new(rect.x, btn_y, BUTTON_W, BUTTON_H);
    let file_rect = Rect::new(rect.x + BUTTON_W + spacing, btn_y, BUTTON_W, BUTTON_H);
    let cancel_rect = Rect::new(
        rect.x + 2.0 * (BUTTON_W + spacing),
        btn_y,
        BUTTON_W,
        BUTTON_H,
    );
    (confirm_rect, file_rect, cancel_rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_content_rect_centers_expected_geometry() {
        let modal = Rect::new(100.0, 60.0, 400.0, 180.0);
        let content = prompt_content_rect(modal, 98.0);

        assert_eq!(content.x, 140.0);
        assert_eq!(content.y, 101.0);
        assert_eq!(content.w, 320.0);
        assert_eq!(content.h, 98.0);
    }

    #[test]
    fn confirm_file_cancel_rects_span_full_prompt_width() {
        let rect = Rect::new(140.0, 100.0, 272.0, 120.0);
        let (open_rect, file_rect, cancel_rect) = three_button_rects(rect, 180.0);

        assert_eq!(open_rect.x, rect.x);
        assert_eq!(cancel_rect.x + cancel_rect.w, rect.x + rect.w);
        assert_eq!(open_rect.w, BUTTON_W);
        assert_eq!(file_rect.w, BUTTON_W);
        assert_eq!(cancel_rect.w, BUTTON_W);
        assert_eq!(file_rect.x - (open_rect.x + open_rect.w), 16.0);
        assert_eq!(cancel_rect.x - (file_rect.x + file_rect.w), 16.0);
    }
}
