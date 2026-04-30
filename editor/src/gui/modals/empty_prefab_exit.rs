use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static EMPTY_PREFAB_EXIT_RESULT: RefCell<Option<EmptyPrefabExitPromptResult>> = const { RefCell::new(None) };
}

pub struct EmptyPrefabExitModal;

crate::register_modal!(EmptyPrefabExitModal);

impl ModalHandler for EmptyPrefabExitModal {
    type Result = EmptyPrefabExitPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EMPTY_PREFAB_EXIT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 560.0, 140.0);
        let mut prompt = EmptyPrefabExitPrompt::new(
            editor.modal.rect,
            "This prefab is empty. What do you want to do?",
        );
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &EMPTY_PREFAB_EXIT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        editor.confirm_empty_prefab_transition(result);
        editor.modal.close();
        None
    }
}
