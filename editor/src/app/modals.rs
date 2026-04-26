use crate::app::*;
use crate::app::escape::modal_escape_requested;
use crate::gui::modals::{
    ModalRegistry, ModalResult, ModalHandler,
};
use crate::gui::modals::new_game::NewGameModal;
use bishop::prelude::*;
use engine_core::prelude::*;

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

    pub(crate) fn duplicate_game_exists(&mut self, name: &str) -> bool {
        let duplicate_exists = list_game_names().iter().any(|existing| existing == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(format!("\"{name}\" already exists."), 2.5));
        };

        duplicate_exists
    }

    pub(crate) fn duplicate_prefab_name_exists_excluding(
        &mut self,
        name: &str,
        exclude_id: PrefabId,
    ) -> bool {
        let duplicate_exists = self
            .game
            .prefab_manager
            .prefabs
            .iter()
            .any(|(&id, prefab)| id != exclude_id && prefab.name == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(
                format!("A prefab named \"{name}\" already exists."),
                2.5,
            ));
        }

        duplicate_exists
    }

    pub(crate) fn current_rename_value(&self) -> String {
        match self.mode {
            EditorMode::Game => self.game.name.clone(),
            EditorMode::World(_) => self.game.current_world().name.clone(),
            EditorMode::Room(id) => self
                .game
                .current_world()
                .get_room(id)
                .map(|room| room.name.clone())
                .unwrap_or_else(|| "Room".to_string()),
            EditorMode::Prefab(_) => self
                .prefab_editor
                .as_ref()
                .map(|editor| editor.prefab_name.clone())
                .unwrap_or_else(|| "Prefab".to_string()),
            EditorMode::Menu => String::new(),
        }
    }
}
