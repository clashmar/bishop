use crate::app::Editor;
use crate::gui::modals::confirm;
use crate::gui::modals::{ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    pub static EXPORT_OVERWRITE_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
    static EXPORT_OVERWRITE_PENDING_MESSAGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn stage_export_overwrite_message(message: String) {
    EXPORT_OVERWRITE_PENDING_MESSAGE.with(|c| *c.borrow_mut() = Some(message));
}

pub struct ExportOverwriteModal;

crate::register_modal!(ExportOverwriteModal);

impl ModalHandler for ExportOverwriteModal {
    type Result = ConfirmPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EXPORT_OVERWRITE_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        let message = EXPORT_OVERWRITE_PENDING_MESSAGE
            .with(|c| c.borrow_mut().take())
            .unwrap_or_default();
        editor.modal =
            confirm::open_confirm_modal_with_message(ctx, &EXPORT_OVERWRITE_RESULT, message);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            ConfirmPromptResult::Confirmed => {
                if let Some(pending_export) = editor.pending_export.take() {
                    editor.finish_export(&pending_export.dest_root);
                }
            }
            ConfirmPromptResult::Cancelled => {
                editor.pending_export = None;
                editor.toast = Some(Toast::new("Export cancelled.", 2.5));
            }
        }
        editor.modal.close();
        None
    }

    fn on_outside_click(&mut self, editor: &mut Editor) {
        if editor.pending_export.take().is_some() {
            EXPORT_OVERWRITE_RESULT.with(|c| *c.borrow_mut() = None);
            editor.toast = Some(Toast::new("Export cancelled.", 2.5));
        }
    }
}
