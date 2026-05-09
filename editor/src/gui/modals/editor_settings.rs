use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::editor_settings_prompt::{EditorSettingsPrompt, EditorSettingsResult};
use bishop::prelude::*;
use engine_core::prelude::WidgetId;
use engine_core::theme::set_theme;
use engine_core::theme::storage::save_editor_preset;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static EDITOR_SETTINGS_RESULT: RefCell<Option<EditorSettingsResult>> = const { RefCell::new(None) };
}

pub struct EditorSettingsModal;

crate::register_modal!(EditorSettingsModal);

impl ModalHandler for EditorSettingsModal {
    type Result = EditorSettingsResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EDITOR_SETTINGS_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 300.0, 150.0);
        let mut prompt = EditorSettingsPrompt::new(editor.modal.rect, WidgetId::default());
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &EDITOR_SETTINGS_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        if result.confirmed {
            if let Some(ref name) = result.preset_name {
                save_editor_preset(name);
            }
        } else {
            set_theme(result.snapshot_theme);
        }
        editor.modal.close();
        None
    }
}
