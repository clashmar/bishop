use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static EMPTY_PREFAB_SAVE_RESULT: RefCell<Option<EmptyPrefabSaveConfirmResult>> = const { RefCell::new(None) };
}

pub struct EmptyPrefabSaveModal;

crate::register_modal!(EmptyPrefabSaveModal);

impl ModalHandler for EmptyPrefabSaveModal {
    type Result = EmptyPrefabSaveConfirmResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EMPTY_PREFAB_SAVE_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 420.0, 140.0);
        let mut prompt = EmptyPrefabSavePrompt::new(
            editor.modal.rect,
            "Saving will delete this prefab and all linked instances.",
        );
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &EMPTY_PREFAB_SAVE_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        if result == EmptyPrefabSaveConfirmResult::Delete {
            editor.confirm_empty_prefab_save_delete();
        }
        editor.modal.close();
        None
    }
}
