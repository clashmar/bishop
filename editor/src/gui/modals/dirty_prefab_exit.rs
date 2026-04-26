use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static DIRTY_PREFAB_EXIT_RESULT: RefCell<Option<DirtyPrefabExitPromptResult>> = const { RefCell::new(None) };
}

pub struct DirtyPrefabExitModal;

crate::register_modal!(DirtyPrefabExitModal);

impl ModalHandler for DirtyPrefabExitModal {
    type Result = DirtyPrefabExitPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &DIRTY_PREFAB_EXIT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 560.0, 140.0);
        let mut prompt =
            DirtyPrefabExitPrompt::new(editor.modal.rect, "Do you want to save prefab changes?");
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &DIRTY_PREFAB_EXIT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        editor.confirm_dirty_prefab_transition(result);
        editor.modal.close();
        None
    }
}
