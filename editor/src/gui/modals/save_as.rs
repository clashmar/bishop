use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use crate::storage::editor_storage::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static SAVE_AS_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

pub struct SaveAsModal;

crate::register_modal!(SaveAsModal);

impl ModalHandler for SaveAsModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &SAVE_AS_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, "Save as:");
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &SAVE_AS_PROMPT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            StringPromptResult::Confirmed(name) => {
                if editor.duplicate_game_exists(&name) {
                    return None;
                }
                match save_as(&mut editor.game, &name) {
                    Ok(()) => editor.save(),
                    Err(err) => {
                        editor.toast = Some(Toast::new(format!("Failed to save game: {err}"), 3.0));
                    }
                }
                editor.modal.close();
            }
            StringPromptResult::Cancelled => {
                editor.modal.close();
            }
        }
        None
    }
}
