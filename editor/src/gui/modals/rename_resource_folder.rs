use crate::app::Editor;
use crate::commands::asset::RenameDirectoryCmd;
use crate::editor_global::push_command;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::{Toast, UserPath};
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static RENAME_FOLDER_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

thread_local! {
    pub(crate) static RENAME_FOLDER_TARGET: RefCell<Option<UserPath>> = const { RefCell::new(None) };
}

pub struct ResourceFolderRenameModal;

crate::register_modal!(ResourceFolderRenameModal);

impl ModalHandler for ResourceFolderRenameModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &RENAME_FOLDER_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        let current_name = RENAME_FOLDER_TARGET.with(|t| {
            t.borrow()
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });

        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, "Rename folder:")
            .with_initial_value(&current_name)
            .select_all_on_open();
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &RENAME_FOLDER_PROMPT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            StringPromptResult::Confirmed(new_name) => {
                if new_name.trim().is_empty() {
                    editor.toast = Some(Toast::new("Name cannot be empty", 2.0));
                    return None;
                }
                let old_path = RENAME_FOLDER_TARGET.with(|t| t.borrow_mut().take());
                if let Some(old) = old_path {
                    if let Some(parent) = old.parent() {
                        let new = parent.join(&new_name);
                        push_command(Box::new(RenameDirectoryCmd::new(old, new)));
                    }
                }
                editor.modal.close();
            }
            StringPromptResult::Cancelled => {
                RENAME_FOLDER_TARGET.with(|t| t.borrow_mut().take());
                editor.modal.close();
            }
        }
        None
    }
}
