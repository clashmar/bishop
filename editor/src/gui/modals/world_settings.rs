use crate::app::Editor;
use crate::commands::world::*;
use crate::editor_global::*;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static WORLD_SETTINGS_RESULT: RefCell<Option<WorldSettingsResult>> = const { RefCell::new(None) };
}

pub struct WorldSettingsModal;

crate::register_modal!(WorldSettingsModal);

impl ModalHandler for WorldSettingsModal {
    type Result = WorldSettingsResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &WORLD_SETTINGS_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        editor.modal = Modal::new(ctx, 300.0, 150.0);
        let world = editor.game.current_world();
        let world_id = world.id;
        let grid_size = world.grid_size;
        let mut prompt =
            WorldSettingsPrompt::new(world_id, editor.modal.rect, WidgetId::default(), grid_size);
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &WORLD_SETTINGS_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        if let Some(new_grid_size) = result.grid_size {
            let old_grid_size = editor.game.get_world_mut(result.id).grid_size;
            push_command(Box::new(ChangeGridSizeCmd::new(
                result.id,
                old_grid_size,
                new_grid_size,
            )));
        }
        editor.modal.close();
        None
    }
}
