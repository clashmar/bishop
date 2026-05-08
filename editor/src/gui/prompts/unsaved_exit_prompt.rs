use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::{draw_prompt_label, prompt_content_rect};
use crate::gui::prompts::PromptFocus;
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedExitResult {
    Save,
    DontSave,
    Cancel,
}

pub struct UnsavedChangesExitPrompt {
    rect: Rect,
    message: String,
    focus: PromptFocus,
}

impl UnsavedChangesExitPrompt {
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        let total_h = PROMPT_TOP_PADDING
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

        Self {
            rect,
            message: message.into(),
            focus: PromptFocus::new(3),
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<UnsavedExitResult> {
        let text_dims = measure_text(ctx, &self.message, layout::DEFAULT_FONT_SIZE_16);
        let x = self.rect.x + (self.rect.w - text_dims.width) * 0.5;
        draw_prompt_label(ctx, &self.message, x, self.rect.y + PROMPT_TOP_PADDING);

        self.focus.navigate(ctx);

        let y =
            self.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_SECTION_GAP;
        let gap = 6.0;
        let width = (self.rect.w - (gap * 2.0)) / 3.0;
        let first = Rect::new(self.rect.x, y, width, BUTTON_H);
        let second = Rect::new(self.rect.x + width + gap, y, width, BUTTON_H);
        let third = Rect::new(self.rect.x + (width + gap) * 2.0, y, width, BUTTON_H);

        if Button::new(first, "Save").focused(self.focus.is_focused(0)).show(ctx)
            || (self.focus.is_focused(0) && Controls::enter(ctx))
        {
            return Some(UnsavedExitResult::Save);
        }
        if Button::new(second, "Don't Save").focused(self.focus.is_focused(1)).show(ctx)
            || (self.focus.is_focused(1) && Controls::enter(ctx))
        {
            return Some(UnsavedExitResult::DontSave);
        }
        if Button::new(third, "Cancel").focused(self.focus.is_focused(2)).show(ctx)
            || (self.focus.is_focused(2) && Controls::enter(ctx))
            || modal_escape_requested()
        {
            return Some(UnsavedExitResult::Cancel);
        }
        None
    }
}