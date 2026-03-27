// editor/src/gui/prompts/helpers.rs
use crate::gui::prompts::constants::*;
use bishop::prelude::*;

/// Supplies centered rects for confirm/cancel buttons.
pub fn confirm_cancel_rects(rect: Rect, btn_y: f32) -> (Rect, Rect) {
    let spacing = (rect.w - 2.0 * BUTTON_W) / 3.0;
    let confirm_rect = Rect::new(rect.x + spacing, btn_y, BUTTON_W, BUTTON_H);
    let cancel_rect = Rect::new(rect.x + 2.0 * spacing + BUTTON_W, btn_y, BUTTON_W, BUTTON_H);
    (confirm_rect, cancel_rect)
}
