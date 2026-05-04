use crate::app::*;
use crate::app::escape::modal_escape_requested;
use crate::gui::modals::{
    ModalRegistry, ModalResult, ModalHandler,
};
use crate::gui::modals::new_game::NewGameModal;
use bishop::prelude::*;

impl Editor {
    /// Returns `Some(name)` when the user confirms, `None` on cancel.
    pub async fn prompt_new_game(&mut self, ctx: PlatformContext) -> Option<String> {
        NewGameModal.open(self, &mut ctx.borrow_mut());

        loop {
            if let Some(ModalResult::String(name)) = self.handle_modal(&mut ctx.borrow_mut()) {
                self.modal.close();
                return Some(name);
            }

            if !self.modal.is_open() {
                return None;
            }

            self.draw_toast(&mut ctx.borrow_mut());

            let next_frame = { ctx.borrow().next_frame() };
            next_frame.await;
        }
    }

    pub(crate) fn register_modal_handlers(&mut self) {
        self.modal_handlers.init_from_inventory();
    }

    pub fn handle_modal(&mut self, ctx: &mut WgpuContext) -> Option<ModalResult> {
        if !self.modal.is_open() {
            return None;
        }

        if self.prefab_state.require_picker() && modal_escape_requested() {
            self.modal.close();
            self.close_active_prefab_editor();
            return None;
        }

        // Draw and check for outside-click
        if self
            .modal
            .draw(ctx, &mut self.game.asset_registry, &mut self.game.sprite_manager)
        {
            if self.should_ignore_modal_clicked_outside() {
                return None;
            }
            return self.handle_outside_click();
        }

        self.handle_modal_results(ctx)
    }

    fn handle_modal_results(&mut self, ctx: &mut WgpuContext) -> Option<ModalResult> {
        let mut handlers = std::mem::replace(&mut self.modal_handlers, ModalRegistry::new());
        let result = handlers.try_handle_all(self, ctx);
        self.modal_handlers = handlers;
        result
    }

    fn handle_outside_click(&mut self) -> Option<ModalResult> {
        let mut handlers = std::mem::replace(&mut self.modal_handlers, ModalRegistry::new());
        handlers.handle_outside_click(self);
        self.modal_handlers = handlers;
        Some(ModalResult::ClickedOutside)
    }

    pub(crate) fn should_ignore_modal_clicked_outside(&self) -> bool {
        self.prefab_state.require_picker() || self.handling_close
    }

}
