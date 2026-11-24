use crate::app::Editor;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::input_bindings_prompt::{
    InputBindingsPrompt, InputBindingsPromptResult,
};
use crate::storage::game_io::save_game;
use bishop::prelude::WgpuContext;
use engine_core::logging::omni_error;
use engine_core::storage::{load_input_settings, save_input_settings};
use engine_core::ui::Toast;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static INPUT_BINDINGS_RESULT: RefCell<Option<InputBindingsPromptResult>> = const { RefCell::new(None) };
}

/// Modal for authoring game actions and editing player input bindings.
pub struct InputBindingsModal;

crate::register_modal!(InputBindingsModal);

impl ModalHandler for InputBindingsModal {
    type Result = InputBindingsPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &INPUT_BINDINGS_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 720.0, 520.0);
        let mut prompt = InputBindingsPrompt::new(
            editor.modal.rect,
            editor.game.input_actions.clone(),
            load_input_settings(),
            editor.game.id,
        );
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &INPUT_BINDINGS_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        if let InputBindingsPromptResult::Save(changes) = result {
            let mut saved = false;
            let mut failed = false;

            if let Some(input_actions) = changes.input_actions {
                editor.game.input_actions = input_actions;
                match save_game(&editor.game) {
                    Ok(()) => {
                        editor.update_save_state_hash();
                        saved = true;
                    }
                    Err(error) => {
                        omni_error!("Could not save game input actions: {error}");
                        failed = true;
                    }
                }
            }

            if let Some(input_settings) = changes.input_settings {
                match save_input_settings(&input_settings) {
                    Ok(()) => saved = true,
                    Err(error) => {
                        omni_error!("Could not save player input settings: {error}");
                        failed = true;
                    }
                }
            }

            if failed {
                editor.toast = Some(Toast::new("Could not save all input bindings", 2.5));
            } else if saved {
                editor.toast = Some(Toast::new("Input bindings saved", 2.5));
            }
        }

        editor.modal.close();
        None
    }
}
