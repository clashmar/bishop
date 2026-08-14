use crate::app::control::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::controls::{Controls};
use widgets::*;
use ::widgets::constants::layout;
use ::widgets::{input_is_focused, request_focus};

/// Result of a string prompt.
#[derive(Debug, PartialEq, Eq)]
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
    /// Whether the current text should be selected when the prompt opens.
    select_all_on_open: bool,
}

impl StringPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        let total_h = PROMPT_TOP_PADDING
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

        Self {
            input_id: WidgetId::default(),
            rect,
            message: message.into(),
            current: String::new(),
            select_all_on_open: false,
        }
    }

    /// Sets the initial text shown in the prompt input.
    pub fn with_initial_value(mut self, value: impl Into<String>) -> Self {
        self.current = value.into();
        self
    }

    /// Selects the initial text when the prompt first opens.
    pub fn select_all_on_open(mut self) -> Self {
        self.select_all_on_open = true;
        self
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<StringPromptResult> {
        self.draw_with_ctx(ctx, Controls::enter(ctx), modal_escape_requested())
    }

    fn draw_with_ctx<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        enter_pressed: bool,
        escape_pressed: bool,
    ) -> Option<StringPromptResult> {
        draw_prompt_label(
            ctx,
            &self.message,
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING,
        );

        let field_rect = Rect::new(
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
            self.rect.w,
            FIELD_H,
        );

        let mut input = TextInput::new(self.input_id, field_rect, &self.current)
            .focused(true);
        if self.select_all_on_open {
            input = input.select_all_on_focus();
        }
        let (new_text, _) = input.show(ctx);
        self.current = new_text;

        let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Handle result
        if (confirm_clicked || enter_pressed) && !self.current.trim().is_empty() {
            return Some(StringPromptResult::Confirmed(self.current.clone()));
        }

        if cancel_clicked || escape_pressed {
            return Some(StringPromptResult::Cancelled);
        }

        if !input_is_focused() {
            request_focus(self.input_id, true);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::prompts::helpers::confirm_cancel_rects;
    use widgets::test_utils::WidgetTestContext;
    use widgets::{clear_click_target, reset_click_consumed};

    fn reset_widget_state() {
        reset_click_consumed();
        clear_click_target(MouseButton::Left);
        text_input_reset(WidgetId::default());
    }

    #[test]
    fn clicking_confirm_submits_the_typed_value() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt = StringPrompt::new(modal_rect, "Enter prefab name:");
        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = WidgetTestContext::new();
        ctx.chars = vec!['C', 'r', 'a', 't', 'e'];
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.chars.clear();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("Crate".to_string()))
        );
    }

    #[test]
    fn confirming_prefilled_prompt_without_typing_returns_initial_value() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt =
            StringPrompt::new(modal_rect, "Rename room:").with_initial_value("Entry Hall");

        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = WidgetTestContext::new();
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("Entry Hall".to_string()))
        );
    }

    #[test]
    fn select_all_on_open_replaces_prefilled_value_when_typing() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt = StringPrompt::new(modal_rect, "Rename prefab:")
            .with_initial_value("Crate")
            .select_all_on_open();

        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = WidgetTestContext::new();
        ctx.chars = vec!['N'];
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.chars.clear();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("N".to_string()))
        );
    }
}
