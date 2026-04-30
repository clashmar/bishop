use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::{draw_prompt_label, prompt_content_rect};
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyPrefabSaveConfirmResult {
    Delete,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyPrefabExitPromptResult {
    DeletePrefab,
    DiscardChanges,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirtyPrefabExitPromptResult {
    SaveAndSync,
    DiscardChanges,
    Cancel,
}

pub struct EmptyPrefabSavePrompt {
    rect: Rect,
    message: String,
}

pub struct EmptyPrefabExitPrompt {
    rect: Rect,
    message: String,
}

pub struct DirtyPrefabExitPrompt {
    rect: Rect,
    message: String,
}

impl EmptyPrefabSavePrompt {
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        Self {
            rect: prefab_prompt_rect(modal_rect),
            message: message.into(),
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<EmptyPrefabSaveConfirmResult> {
        draw_prefab_prompt_message(ctx, self.rect, &self.message);
        let (left, right) = prefab_two_button_rects(self.rect);
        if Button::new(left, "Delete Prefab").show(ctx) || Controls::enter(ctx) {
            return Some(EmptyPrefabSaveConfirmResult::Delete);
        }
        if Button::new(right, "Cancel").show(ctx) || modal_escape_requested() {
            return Some(EmptyPrefabSaveConfirmResult::Cancel);
        }
        None
    }
}

impl EmptyPrefabExitPrompt {
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        Self {
            rect: prefab_prompt_rect(modal_rect),
            message: message.into(),
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<EmptyPrefabExitPromptResult> {
        draw_prefab_prompt_message(ctx, self.rect, &self.message);
        let (first, second, third) = prefab_three_button_rects(self.rect);
        if Button::new(first, "Delete Prefab").show(ctx) {
            return Some(EmptyPrefabExitPromptResult::DeletePrefab);
        }
        if Button::new(second, "Discard").show(ctx) || Controls::enter(ctx) {
            return Some(EmptyPrefabExitPromptResult::DiscardChanges);
        }
        if Button::new(third, "Cancel").show(ctx) || modal_escape_requested() {
            return Some(EmptyPrefabExitPromptResult::Cancel);
        }
        None
    }
}

impl DirtyPrefabExitPrompt {
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        Self {
            rect: prefab_prompt_rect(modal_rect),
            message: message.into(),
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<DirtyPrefabExitPromptResult> {
        draw_prefab_prompt_message(ctx, self.rect, &self.message);
        let (first, second, third) = prefab_three_button_rects(self.rect);
        if Button::new(first, "Save and Sync").show(ctx) || Controls::enter(ctx) {
            return Some(DirtyPrefabExitPromptResult::SaveAndSync);
        }
        if Button::new(second, "Discard").show(ctx) {
            return Some(DirtyPrefabExitPromptResult::DiscardChanges);
        }
        if Button::new(third, "Cancel").show(ctx) || modal_escape_requested() {
            return Some(DirtyPrefabExitPromptResult::Cancel);
        }
        None
    }
}

fn prefab_prompt_rect(modal_rect: Rect) -> Rect {
    let total_h = PROMPT_TOP_PADDING
        + layout::DEFAULT_FONT_SIZE_16
        + PROMPT_SECTION_GAP
        + BUTTON_H
        + PROMPT_BOTTOM_PADDING;
    prompt_content_rect(modal_rect, total_h)
}

fn draw_prefab_prompt_message(ctx: &mut WgpuContext, rect: Rect, message: &str) {
    let text_dims = measure_text(ctx, message, layout::DEFAULT_FONT_SIZE_16);
    let x = rect.x + (rect.w - text_dims.width) * 0.5;
    draw_prompt_label(ctx, message, x, rect.y + PROMPT_TOP_PADDING);
}

fn prefab_two_button_rects(rect: Rect) -> (Rect, Rect) {
    let y = rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_SECTION_GAP;
    let gap = 12.0;
    let width = (rect.w - gap) * 0.5;
    (
        Rect::new(rect.x, y, width, BUTTON_H),
        Rect::new(rect.x + width + gap, y, width, BUTTON_H),
    )
}

fn prefab_three_button_rects(rect: Rect) -> (Rect, Rect, Rect) {
    let y = rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_SECTION_GAP;
    let gap = 12.0;
    let width = (rect.w - (gap * 2.0)) / 3.0;
    (
        Rect::new(rect.x, y, width, BUTTON_H),
        Rect::new(rect.x + width + gap, y, width, BUTTON_H),
        Rect::new(rect.x + (width + gap) * 2.0, y, width, BUTTON_H),
    )
}
