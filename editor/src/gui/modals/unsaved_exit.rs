use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static UNSAVED_EXIT_RESULT: RefCell<Option<UnsavedExitResult>> = const { RefCell::new(None) };
}

pub struct UnsavedExitModal;

crate::register_modal!(UnsavedExitModal);

impl ModalHandler for UnsavedExitModal {
    type Result = UnsavedExitResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &UNSAVED_EXIT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 448.0, 140.0);
        let mut prompt = UnsavedChangesExitPrompt::new(
            editor.modal.rect,
            "Do you want to save changes before exiting?",
        );
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &UNSAVED_EXIT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            UnsavedExitResult::Save => {
                editor.save();
                ctx.set_exit_confirmed(true);
            }
            UnsavedExitResult::DontSave => {
                ctx.set_exit_confirmed(true);
            }
            UnsavedExitResult::Cancel => {
                ctx.set_close_requested(false);
            }
        }
        editor.handling_close = false;
        editor.modal.close();
        None
    }
}
