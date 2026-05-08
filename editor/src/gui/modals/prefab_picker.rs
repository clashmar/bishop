use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use crate::prefab::PrefabTransitionPrompt;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static PREFAB_PICKER_RESULT: RefCell<Option<PrefabPickerResult>> = const { RefCell::new(None) };
}

pub struct PrefabPickerModal;

crate::register_modal!(PrefabPickerModal);

impl ModalHandler for PrefabPickerModal {
    type Result = PrefabPickerResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &PREFAB_PICKER_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 340.0, 240.0);
        let excluded_prefab_id = editor.active_persisted_prefab_id();
        let prefabs = editor.game.prefab_manager.sorted_prefabs();
        let mut prompt = PrefabPickerPrompt::new(
            editor.modal.rect,
            prefabs,
            excluded_prefab_id,
            editor.prefab_state.require_picker(),
        );
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &PREFAB_PICKER_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            PrefabPickerResult::Existing(prefab) => {
                let prompt = editor.request_prefab_transition_to_asset(prefab);
                if prompt == PrefabTransitionPrompt::None {
                    editor.modal.close();
                }
                editor.present_prefab_transition_prompt(ctx, prompt);
            }
            PrefabPickerResult::New(path) => {
                let target = editor.resolve_initial_prefab_save_target(path);
                let target = target?;
                let prompt = editor.request_blank_prefab_transition(target.name, target.path);
                if prompt == PrefabTransitionPrompt::None {
                    editor.modal.close();
                }
                editor.present_prefab_transition_prompt(ctx, prompt);
            }
            PrefabPickerResult::File(path) => {
                match editor.request_prefab_transition_to_path(&path) {
                    Ok(prompt) => {
                        if prompt == PrefabTransitionPrompt::None {
                            editor.modal.close();
                        }
                        editor.present_prefab_transition_prompt(ctx, prompt);
                    }
                    Err(error) => {
                        onscreen_error!("Could not open prefab: {error}");
                    }
                }
            }
            PrefabPickerResult::Cancelled => {
                if editor.prefab_state.require_picker() {
                    editor.modal.close();
                    editor.close_active_prefab_editor();
                    return None;
                }
                editor.modal.close();
            }
        }
        None
    }

    fn on_outside_click(&mut self, _editor: &mut Editor) {
        // Explicit cancel only.
    }
}
