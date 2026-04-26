use crate::app::Editor;
use crate::commands::asset::{DeleteAssetCmd, DeleteDirectoryCmd, DeleteUnregisteredFileCmd};
use crate::editor_global::push_command;
use crate::gui::modals::confirm;
use crate::gui::modals::{take_modal_result, ModalHandler, ModalResult};
use crate::gui::panels::resources_panel::context_menu::PendingResourceAction;
use crate::gui::prompts::ConfirmPromptResult;
use bishop::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    pub static DELETE_RESOURCE_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
    pub static DELETE_RESOURCE_TARGET: RefCell<Option<PendingResourceAction>> = const { RefCell::new(None) };
}

pub struct DeleteResourceModal;

crate::register_modal!(DeleteResourceModal);

impl ModalHandler for DeleteResourceModal {
    type Result = ConfirmPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &DELETE_RESOURCE_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        if DELETE_RESOURCE_TARGET.with(|c| c.borrow().is_none()) {
            return;
        }
        editor.modal = confirm::open_confirm_modal(ctx, &DELETE_RESOURCE_RESULT);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            ConfirmPromptResult::Confirmed => {
                if let Some(action) = DELETE_RESOURCE_TARGET.with(|c| c.borrow_mut().take()) {
                    match action {
                        PendingResourceAction::DeleteRegisteredFile(key) => {
                            push_command(Box::new(DeleteAssetCmd::new(key)));
                        }
                        PendingResourceAction::DeleteUnregisteredFile(path) => {
                            push_command(Box::new(DeleteUnregisteredFileCmd::new(path)));
                        }
                        PendingResourceAction::DeleteDirectory(user_path) => {
                            push_command(Box::new(DeleteDirectoryCmd::new(user_path)));
                        }
                        _ => {}
                    }
                }
            }
            ConfirmPromptResult::Cancelled => {
                DELETE_RESOURCE_TARGET.with(|c| c.borrow_mut().take());
            }
        }
        editor.modal.close();
        None
    }

    fn on_outside_click(&mut self, _editor: &mut Editor) {
        DELETE_RESOURCE_TARGET.with(|c| c.borrow_mut().take());
        take_modal_result(self.result_store());
    }
}
