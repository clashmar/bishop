use crate::app::Editor;
use crate::commands::game::*;
use crate::editor_global::*;
use crate::gui::modals::{BoxedWidget, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;
use widgets::WidgetId;

thread_local! {
    pub static EDIT_WORLD_RESULT: RefCell<Option<WorldEditResult>> = const { RefCell::new(None) };
}

pub struct EditWorldData {
    pub world_id: WorldId,
    pub current_name: String,
    pub current_sprite: SpriteId,
    pub widget_id: WidgetId,
}

pub struct EditWorldModal;

crate::register_modal!(EditWorldModal);

impl ModalHandler for EditWorldModal {
    type Result = WorldEditResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &EDIT_WORLD_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        let Some(data) = editor.game_editor.pending_edit_world.take() else {
            return;
        };

        editor.modal = Modal::new(ctx, 400.0, 300.0);
        let mut prompt = WorldEditPrompt::new(
            data.world_id,
            editor.modal.rect,
            data.widget_id,
            data.current_name,
            data.current_sprite,
        );

        let widgets: Vec<BoxedWidget> =
            vec![Box::new(move |ctx, asset_registry, sprite_manager| {
                if let Some(result) = prompt.draw(ctx, asset_registry, sprite_manager) {
                    EDIT_WORLD_RESULT.with(|c| *c.borrow_mut() = Some(result));
                }
            })];

        editor.modal.open(widgets);
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        let mut new_name = None;
        let mut new_sprite = None;

        if let Some(ref name) = result.name {
            if let Some(world) = editor.game.get_world(result.id) {
                if world.name != *name {
                    new_name = Some(name.clone());
                }
            }
        }

        if let Some(sprite) = result.sprite {
            let sprite_opt = if sprite.0 == 0 { None } else { Some(sprite) };
            if let Some(world) = editor.game.get_world(result.id) {
                if world.meta.sprite_id != sprite_opt {
                    new_sprite = Some(sprite_opt);
                }
            }
        }

        if new_name.is_some() || new_sprite.is_some() {
            push_command(Box::new(EditWorldCmd::new(result.id, new_name, new_sprite)));
        }
        editor.modal.close();
        None
    }
}
