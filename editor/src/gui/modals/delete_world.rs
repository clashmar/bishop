use crate::app::Editor;
use crate::commands::game::*;
use crate::editor_global::*;
use crate::gui::modals::confirm;
use crate::gui::modals::{take_modal_result, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::worlds::world::WorldId;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    pub static DELETE_WORLD_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
    pub static DELETE_WORLD_TARGET: RefCell<Option<WorldId>> = const { RefCell::new(None) };
}

pub struct DeleteWorldModal;

crate::register_modal!(DeleteWorldModal);

impl ModalHandler for DeleteWorldModal {
    type Result = ConfirmPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &DELETE_WORLD_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        let Some(world_id) = editor.game_editor.pending_delete_world.take() else {
            return;
        };
        DELETE_WORLD_TARGET.with(|c| *c.borrow_mut() = Some(world_id));
        editor.modal = confirm::open_confirm_modal(ctx, &DELETE_WORLD_RESULT);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            ConfirmPromptResult::Confirmed => {
                if let Some(world_id) = DELETE_WORLD_TARGET.with(|c| c.borrow_mut().take()) {
                    push_command(Box::new(DeleteWorldCmd::new(&mut editor.game, world_id)));
                }
            }
            ConfirmPromptResult::Cancelled => {
                DELETE_WORLD_TARGET.with(|c| c.borrow_mut().take());
            }
        }
        editor.modal.close();
        None
    }

    fn on_outside_click(&mut self, _editor: &mut Editor) {
        DELETE_WORLD_TARGET.with(|c| c.borrow_mut().take());
        take_modal_result(self.result_store());
    }
}
