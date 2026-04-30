use crate::app::Editor;
use crate::commands::asset::RenameAssetCmd;
use crate::editor_global::push_command;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::thread::LocalKey;

thread_local! {
    static RENAME_RESOURCE_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

thread_local! {
    pub(crate) static RENAME_RESOURCE_TARGET: RefCell<Option<(AssetKey, PathBuf)>> = const { RefCell::new(None) };
}

pub struct ResourceRenameModal;

crate::register_modal!(ResourceRenameModal);

impl ModalHandler for ResourceRenameModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &RENAME_RESOURCE_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        let current_name = RENAME_RESOURCE_TARGET.with(|t| {
            t.borrow()
                .as_ref()
                .and_then(|(_, old_relative)| old_relative.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });

        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, "Rename resource:")
            .with_initial_value(&current_name)
            .select_all_on_open();
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &RENAME_RESOURCE_PROMPT_RESULT,
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
                let target = RENAME_RESOURCE_TARGET.with(|t| t.borrow_mut().take());
                if let Some((key, old_relative)) = target {
                    let new_relative = if let Some(parent) = old_relative.parent() {
                        parent.join(&new_name)
                    } else {
                        PathBuf::from(&new_name)
                    };
                    push_command(Box::new(RenameAssetCmd::new(key, new_relative)));
                }
                editor.modal.close();
            }
            StringPromptResult::Cancelled => {
                RENAME_RESOURCE_TARGET.with(|t| t.borrow_mut().take());
                editor.modal.close();
            }
        }
        None
    }
}
