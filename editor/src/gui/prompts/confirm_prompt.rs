// editor/src/gui/prompts/confirm_prompt.rs
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Result of a confirm prompt.
pub enum ConfirmPromptResult {
    Confirmed,
    Cancelled,
}

/// A prompt that draws:
///   * Message line,
///   * Confirm / Cancel buttons.
pub struct ConfirmPrompt {
    /// Rectangle that contains the whole widget.
    rect: Rect,
    /// Message to display.
    message: String,
}

impl ConfirmPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        let inner_w = modal_rect.w * 0.8;
        let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;

        let total_h = modal_rect.h * 1.3;
        let inner_y = modal_rect.y + (total_h - modal_rect.h);

        let rect = Rect::new(inner_x, inner_y, inner_w, total_h);

        Self {
            rect,
            message: message.into(),
        }
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<ConfirmPromptResult> {
        // Message
        let center_x = self.rect.x + (self.rect.w / 2.0);
        let message_x = center_text(ctx, center_x, &self.message, DEFAULT_FONT_SIZE_16).0;
        let message_height = measure_text(ctx, &self.message, DEFAULT_FONT_SIZE_16).height;

        ctx.draw_text(
            &self.message,
            message_x,
            self.rect.y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        // Buttons
        let btn_y = self.rect.y + message_height + WIDGET_SPACING;
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Handle result
        if confirm_clicked || Controls::enter(ctx) {
            return Some(ConfirmPromptResult::Confirmed);
        }

        if cancel_clicked || Controls::escape(ctx) {
            return Some(ConfirmPromptResult::Cancelled);
        }

        None
    }
}
