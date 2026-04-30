use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use crate::storage::editor_storage::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static NEW_GAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

pub struct NewGameModal;

crate::register_modal!(NewGameModal);

impl ModalHandler for NewGameModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &NEW_GAME_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, "Enter game name:");
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &NEW_GAME_PROMPT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            StringPromptResult::Confirmed(name) => {
                if name.trim().is_empty() {
                    editor.toast = Some(Toast::new("Name cannot be empty", 2.0));
                } else if !editor.duplicate_game_exists(&name) {
                    let new_game = create_new_game(name.clone());
                    editor.reset(ctx, new_game);
                    editor.modal.close();
                    return Some(ModalResult::String(name));
                }
            }
            StringPromptResult::Cancelled => {
                editor.modal.close();
            }
        }
        None
    }
}
