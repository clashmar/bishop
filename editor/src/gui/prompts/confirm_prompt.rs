// editor/src/gui/prompts/confirm_prompt.rs
use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
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
        let total_h = PROMPT_TOP_PADDING
            + DEFAULT_FONT_SIZE_16
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

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

        draw_prompt_label(
            ctx,
            &self.message,
            message_x,
            self.rect.y + PROMPT_TOP_PADDING,
        );

        // Buttons
        let btn_y = self.rect.y + PROMPT_TOP_PADDING + message_height + PROMPT_SECTION_GAP;
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Handle result
        if confirm_clicked || Controls::enter(ctx) {
            return Some(ConfirmPromptResult::Confirmed);
        }

        if cancel_clicked || modal_escape_requested() {
            return Some(ConfirmPromptResult::Cancelled);
        }

        None
    }
}
