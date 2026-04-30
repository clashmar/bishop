use crate::app::Editor;
use crate::commands::asset::CreateDirectoryCmd;
use crate::editor_global::push_command;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::Toast;
use std::cell::RefCell;
use std::path::PathBuf;
use std::thread::LocalKey;

thread_local! {
    static NEW_FOLDER_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

thread_local! {
    pub(crate) static NEW_FOLDER_TARGET: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub struct NewResourceFolderModal;

crate::register_modal!(NewResourceFolderModal);

impl ModalHandler for NewResourceFolderModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &NEW_FOLDER_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, "Folder name:");
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &NEW_FOLDER_PROMPT_RESULT,
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
                if name.trim().is_empty() {
                    editor.toast = Some(Toast::new("Name cannot be empty", 2.0));
                    return None;
                }
                let parent = NEW_FOLDER_TARGET.with(|t| t.borrow_mut().take());
                if let Some(p) = parent {
                    push_command(Box::new(CreateDirectoryCmd::new(p.join(&name))));
                }
                editor.modal.close();
            }
            StringPromptResult::Cancelled => {
                NEW_FOLDER_TARGET.with(|t| t.borrow_mut().take());
                editor.modal.close();
            }
        }
        None
    }
}
