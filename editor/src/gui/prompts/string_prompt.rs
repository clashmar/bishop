// editor/src/gui/prompts/string_prompt.rs
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Result of a string prompt.
pub enum StringPromptResult {
    Confirmed(String),
    Cancelled,
}

/// A prompt that draws:
///   * Message line,
///   * Text field,
///   * Confirm / Cancel buttons.
pub struct StringPrompt {
    /// Unique id for the text field.
    input_id: WidgetId,
    /// Rectangle that contains the whole widget.
    rect: Rect,
    /// Message shown above the text field.
    message: String,
    /// Current contents of the text field.
    current: String,
}

impl StringPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        const GAP: f32 = 10.0;

        let inner_w = modal_rect.w * 0.8;
        let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;

        let total_h = 20.0 + FIELD_H + GAP + BUTTON_H;
        let inner_y = modal_rect.y + (modal_rect.h - total_h) / 2.0;

        let rect = Rect::new(inner_x, inner_y, inner_w, total_h);

        Self {
            input_id: WidgetId::default(),
            rect,
            message: message.into(),
            current: String::new(),
        }
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<StringPromptResult> {
        let message_pos = vec2(self.rect.x, self.rect.y + 10.0);

        ctx.draw_text(
            &self.message,
            message_pos.x,
            message_pos.y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        // Text field
        let field_rect = Rect::new(self.rect.x, self.rect.y + 20.0, self.rect.w, 30.0);

        let (new_text, _) = TextInput::new(self.input_id, field_rect, &self.current)
            .focused(true)
            .show(ctx);
        self.current = new_text;

        // Buttons
        let btn_y = self.rect.y + 70.0;
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Handle result
        if (confirm_clicked || Controls::enter(ctx)) && !self.current.trim().is_empty() {
            return Some(StringPromptResult::Confirmed(self.current.clone()));
        }

        if cancel_clicked || Controls::escape(ctx) {
            return Some(StringPromptResult::Cancelled);
        }

        None
    }
}
