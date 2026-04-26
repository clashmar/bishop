use crate::gui::modals::{BoxedWidget, Modal};
use crate::gui::prompts::confirm_prompt::*;
use crate::gui::prompts::constants::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::{cell::RefCell, thread::LocalKey};

pub fn open_confirm_modal(
    ctx: &WgpuContext,
    result_store: &'static LocalKey<RefCell<Option<ConfirmPromptResult>>>,
) -> Modal {
    open_confirm_modal_with_message(ctx, result_store, "Are You Sure?")
}

pub fn open_confirm_modal_with_message(
    ctx: &WgpuContext,
    result_store: &'static LocalKey<RefCell<Option<ConfirmPromptResult>>>,
    prompt_message: impl Into<String>,
) -> Modal {
    let prompt_message = prompt_message.into();
    let mut modal = Modal::new(ctx, confirm_modal_width(ctx, &prompt_message), 120.0);
    let mut prompt = ConfirmPrompt::new(modal.rect, prompt_message);

    let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
        if let Some(result) = prompt.draw(ctx) {
            result_store.with(|c| *c.borrow_mut() = Some(result));
        }
    })];

    modal.open(widgets);
    modal
}

fn confirm_modal_width(ctx: &WgpuContext, message: &str) -> f32 {
    const MIN_MODAL_WIDTH: f32 = 300.0;
    const SCREEN_MARGIN: f32 = 40.0;
    const MESSAGE_WIDTH_BUFFER: f32 = 48.0;

    let minimum_button_layout_width =
        (BUTTON_W * 2.0 + PROMPT_ACTION_GAP * 3.0) / PROMPT_CONTENT_WIDTH_RATIO;
    let minimum_modal_width = MIN_MODAL_WIDTH.max(minimum_button_layout_width);

    let message_width = measure_text(ctx, message, DEFAULT_FONT_SIZE_16).width;
    let content_width = message_width + MESSAGE_WIDTH_BUFFER;
    let modal_width = content_width / PROMPT_CONTENT_WIDTH_RATIO;
    let max_modal_width = (ctx.screen_width() - SCREEN_MARGIN).max(minimum_modal_width);

    modal_width.clamp(minimum_modal_width, max_modal_width)
}
