use crate::app::Editor;
use crate::gui::modals::confirm;
use crate::gui::modals::{ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    pub static DELETE_PREFAB_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
}

pub struct DeletePrefabModal;

crate::register_modal!(DeletePrefabModal);

impl ModalHandler for DeletePrefabModal {
    type Result = ConfirmPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &DELETE_PREFAB_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = confirm::open_confirm_modal_with_message(
            ctx,
            &DELETE_PREFAB_RESULT,
            "Delete this prefab and all linked room instances?",
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        if matches!(result, ConfirmPromptResult::Confirmed) {
            editor.confirm_delete_prefab();
        }
        editor.modal.close();
        None
    }
}
