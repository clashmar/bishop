use crate::app::*;
use crate::app::escape::modal_escape_requested;
use crate::commands::game::*;
use crate::commands::world::*;
use crate::editor_global::*;
use crate::gui::modal::*;
use crate::gui::prompts::*;
use crate::prefab::{PendingPrefabRequest, PrefabTransitionPrompt};
use crate::storage::editor_storage::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;

impl Editor {
    /// Returns `Some(name)` when the user confirms, `None` on cancel.
    pub async fn prompt_new_game(&mut self, ctx: PlatformContext) -> Option<String> {
        self.open_new_game_modal(&mut ctx.borrow_mut());

        // Wait until the user has responded
        loop {
            // Draws and handles result
            if let Some(ModalResult::String(name)) = self.handle_modal(&mut ctx.borrow_mut()) {
                // Only close modal if a name is returned
                self.modal.close();
                return Some(name);
            }

            // Guard against modal not being open for some reason
            if !self.modal.is_open() {
                return None;
            }

            // Toasts can be created by the prompt
            self.draw_toast(&mut ctx.borrow_mut());

            let next_frame = { ctx.borrow().next_frame() };
            next_frame.await;
        }
    }

    pub(super) fn open_new_game_modal(&mut self, ctx: &mut WgpuContext) {
        let prompt_message = "Enter game name:";
        let mut prompt = self.set_prompt_modal(ctx, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _sprite_manager| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                NEW_GAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub(super) fn open_rename_modal(&mut self, ctx: &mut WgpuContext) {
        if self.is_blank_prefab_mode() {
            self.toast = Some(Toast::new("Blank prefab sessions cannot be renamed.", 2.5));
            return;
        }

        let prompt_message = match self.mode {
            EditorMode::Game => "Rename game: ",
            EditorMode::World(_) => "Rename world: ",
            EditorMode::Room(_) => "Rename room: ",
            EditorMode::Prefab(_) => "Rename prefab: ",
            EditorMode::Menu => "Rename menu: ",
        };

        let mut prompt = self
            .set_prompt_modal(ctx, prompt_message)
            .with_initial_value(self.current_rename_value())
            .select_all_on_open();

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                RENAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub(super) fn open_save_as_modal(&mut self, ctx: &mut WgpuContext) {
        let prompt_message = "Save as:";
        let mut prompt = self.set_prompt_modal(ctx, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                SAVE_AS_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub(crate) fn open_prefab_name_modal(&mut self, ctx: &mut WgpuContext) {
        let mut prompt = self.set_prompt_modal(ctx, "Enter prefab name:");

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                PREFAB_NAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub(crate) fn open_prefab_picker_modal(&mut self, ctx: &mut WgpuContext) {
        self.modal = Modal::new(ctx, 340.0, 240.0);
        let excluded_prefab_id = self.active_persisted_prefab_id();
        let prefabs = list_prefabs(&self.game.name).unwrap_or_default();
        let mut prompt = PrefabPickerPrompt::new(
            self.modal.rect,
            prefabs,
            excluded_prefab_id,
            self.prefab_state.require_picker(),
        );

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                PREFAB_PICKER_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    fn set_prompt_modal(&mut self, ctx: &mut WgpuContext, prompt_message: &str) -> StringPrompt {
        self.modal = Modal::new(ctx, 400.0, 180.0);
        StringPrompt::new(self.modal.rect, prompt_message)
    }

    fn current_rename_value(&self) -> String {
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

    pub(super) fn open_world_settings_modal(&mut self, ctx: &mut WgpuContext) {
        self.modal = Modal::new(ctx, 300.0, 150.0);
        let world = self.game.current_world();
        let world_id = world.id;
        let grid_size = world.grid_size;

        let mut prompt =
            WorldSettingsPrompt::new(world_id, self.modal.rect, WidgetId::default(), grid_size);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                WORLD_SETTINGS_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub(super) fn open_export_overwrite_modal(
        &mut self,
        ctx: &WgpuContext,
        target_path: &std::path::Path,
    ) {
        let target_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("export");
        let message = format!("Overwrite existing export '{target_name}'?");
        self.modal = Modal::open_confirm_modal_with_message(ctx, &EXPORT_OVERWRITE_RESULT, message);
    }

    pub(super) fn open_empty_prefab_save_modal(&mut self, ctx: &WgpuContext) {
        self.modal = Modal::new(ctx, 420.0, 140.0);
        let mut prompt = EmptyPrefabSavePrompt::new(
            self.modal.rect,
            "Saving will delete this prefab and all linked instances.",
        );
        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                EMPTY_PREFAB_SAVE_RESULT.with(|cell| *cell.borrow_mut() = Some(result));
            }
        })];
        self.modal.open(widgets);
    }

    pub(crate) fn open_empty_prefab_exit_modal(&mut self, ctx: &WgpuContext) {
        self.modal = Modal::new(ctx, 560.0, 140.0);
        let mut prompt = EmptyPrefabExitPrompt::new(
            self.modal.rect,
            "This prefab is empty. What do you want to do?",
        );
        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                EMPTY_PREFAB_EXIT_RESULT.with(|cell| *cell.borrow_mut() = Some(result));
            }
        })];
        self.modal.open(widgets);
    }

    pub(crate) fn open_delete_prefab_modal(&mut self, ctx: &WgpuContext) {
        self.modal = Modal::open_confirm_modal_with_message(
            ctx,
            &DELETE_PREFAB_RESULT,
            "Delete this prefab and all linked room instances?",
        );
    }

    pub(crate) fn open_dirty_prefab_exit_modal(&mut self, ctx: &WgpuContext) {
        self.modal = Modal::new(ctx, 560.0, 140.0);
        let mut prompt =
            DirtyPrefabExitPrompt::new(self.modal.rect, "Do you want to save prefab changes?");
        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
            if let Some(result) = prompt.draw(ctx) {
                DIRTY_PREFAB_EXIT_RESULT.with(|cell| *cell.borrow_mut() = Some(result));
            }
        })];
        self.modal.open(widgets);
    }

    pub fn handle_modal(&mut self, ctx: &mut WgpuContext) -> Option<ModalResult> {
        if self.modal.is_open() {
            if self.prefab_state.require_picker() && modal_escape_requested() {
                self.modal.close();
                self.close_active_prefab_editor();
                return None;
            }

            // Outside‑click handling
            if self
                .modal
                .draw(ctx, &mut self.game.asset_registry, &mut self.game.sprite_manager)
            {
                if self.should_ignore_modal_clicked_outside() {
                    return None;
                }
                if self.pending_export.take().is_some() {
                    EXPORT_OVERWRITE_RESULT.with(|c| *c.borrow_mut() = None);
                    self.toast = Some(Toast::new("Export cancelled.", 2.5));
                }
                // Clear any pending results
                NEW_GAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = None);
                return Some(ModalResult::ClickedOutside);
            }

            // New game name prompt
            let new_game_prompt_opt = NEW_GAME_PROMPT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = new_game_prompt_opt {
                match result {
                    StringPromptResult::Confirmed(name) => {
                        // Validation
                        if name.trim().is_empty() {
                            self.toast = Some(Toast::new("Name cannot be empty", 2.0));
                            return None;
                        } else {
                            if self.duplicate_game_exists(&name) {
                                return None;
                            }
                            // Create the new game
                            let new_game = create_new_game(name.clone());
                            self.reset(ctx, new_game);
                            self.modal.close();
                            return Some(ModalResult::String(name));
                        }
                    }
                    StringPromptResult::Cancelled => {
                        self.modal.close();
                        return None;
                    }
                }
            }

            // Rename game name prompt
            let rename_prompt_opt = RENAME_PROMPT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = rename_prompt_opt {
                match result {
                    StringPromptResult::Confirmed(name) => {
                        match self.mode {
                            EditorMode::Game => {
                                if self.duplicate_game_exists(&name) {
                                    return None;
                                }
                                push_command(Box::new(RenameGameCmd::new(
                                    name,
                                    self.game.name.clone(),
                                )))
                            }
                            EditorMode::World(_) => self.game.current_world_mut().name = name,
                            EditorMode::Room(id) => {
                                if let Some(room) = self.game.current_world_mut().get_room_mut(id) {
                                    room.name = name;
                                }
                            }
                            EditorMode::Prefab(_) => {
                                if let Some(prefab_id) = self.active_persisted_prefab_id() {
                                    let is_duplicate = self
                                        .duplicate_prefab_name_exists_excluding(&name, prefab_id);
                                    if !is_duplicate {
                                        if let Some(prefab_editor) = self.prefab_editor.as_mut() {
                                            prefab_editor.set_name(name);
                                        }
                                    }
                                }
                            }
                            EditorMode::Menu => {}
                        }
                        self.modal.close();
                    }
                    StringPromptResult::Cancelled => {
                        self.modal.close();
                        return None;
                    }
                }
            }

            // Save as prompt
            let save_as_prompt_opt = SAVE_AS_PROMPT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = save_as_prompt_opt {
                match result {
                    StringPromptResult::Confirmed(name) => {
                        if self.duplicate_game_exists(&name) {
                            return None;
                        }
                        match save_as(&mut self.game, &name) {
                            Ok(()) => self.save(),
                            Err(err) => {
                                self.toast =
                                    Some(Toast::new(format!("Failed to save game: {err}"), 3.0));
                            }
                        }
                        self.modal.close();
                    }
                    StringPromptResult::Cancelled => {
                        self.modal.close();
                        return None;
                    }
                }
            }

            let prefab_name_prompt_opt = PREFAB_NAME_PROMPT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = prefab_name_prompt_opt {
                enum PrefabNameConfirmOutcome {
                    Close,
                    PresentTransition(PrefabTransitionPrompt),
                }

                match result {
                    StringPromptResult::Confirmed(name) => {
                        if self.duplicate_prefab_name_exists(&name) {
                            self.prefab_state.clear_pending_request();
                            self.modal.close();
                        } else {
                            let outcome = match self.prefab_state.take_pending_request() {
                                Some(PendingPrefabRequest::CaptureSelection(entity)) => {
                                    self.create_prefab_from_selection(ctx, entity, name);
                                    PrefabNameConfirmOutcome::Close
                                }
                                Some(PendingPrefabRequest::CreateBlank) => {
                                    PrefabNameConfirmOutcome::PresentTransition(
                                        self.request_blank_prefab_transition(name),
                                    )
                                }
                                None => {
                                    PrefabNameConfirmOutcome::Close
                                }
                            };

                            match outcome {
                                PrefabNameConfirmOutcome::Close
                                | PrefabNameConfirmOutcome::PresentTransition(
                                    PrefabTransitionPrompt::None,
                                ) => {
                                    self.modal.close();
                                }
                                PrefabNameConfirmOutcome::PresentTransition(prompt) => {
                                    self.present_prefab_transition_prompt(ctx, prompt);
                                }
                            }
                        }
                    }
                    StringPromptResult::Cancelled => {
                        self.prefab_state.clear_pending_request();
                        self.modal.close();
                        return None;
                    }
                }
            }

            let prefab_picker_opt = PREFAB_PICKER_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = prefab_picker_opt {
                match result {
                    PrefabPickerResult::Existing(prefab) => {
                        let prompt = self.request_prefab_transition_to_asset(prefab);
                        if prompt == PrefabTransitionPrompt::None {
                            self.modal.close();
                        }
                        self.present_prefab_transition_prompt(ctx, prompt);
                    }
                    PrefabPickerResult::New => {
                        self.prefab_state
                            .set_pending_request(PendingPrefabRequest::CreateBlank);
                        self.open_prefab_name_modal(ctx);
                        return None;
                    }
                    PrefabPickerResult::File(path) => {
                        match self.request_prefab_transition_to_path(&path) {
                            Ok(prompt) => {
                                if prompt == PrefabTransitionPrompt::None {
                                    self.modal.close();
                                }
                                self.present_prefab_transition_prompt(ctx, prompt);
                            }
                            Err(error) => {
                                onscreen_error!("Could not open prefab: {error}");
                                return None;
                            }
                        }
                    }
                    PrefabPickerResult::Cancelled => {
                        if self.prefab_state.require_picker() {
                            self.modal.close();
                            self.close_active_prefab_editor();
                            return None;
                        }
                        self.modal.close();
                        return None;
                    }
                }
            }

            // World settings prompt
            let world_settings_opt = WORLD_SETTINGS_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = world_settings_opt {
                if let Some(new_grid_size) = result.grid_size {
                    let old_grid_size = self.game.get_world_mut(result.id).grid_size;
                    push_command(Box::new(ChangeGridSizeCmd::new(
                        result.id,
                        old_grid_size,
                        new_grid_size,
                    )));
                }
                self.modal.close();
            }

            let export_overwrite_opt = EXPORT_OVERWRITE_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = export_overwrite_opt {
                match result {
                    ConfirmPromptResult::Confirmed => {
                        if let Some(pending_export) = self.pending_export.take() {
                            self.finish_export(&pending_export.dest_root);
                        }
                    }
                    ConfirmPromptResult::Cancelled => {
                        self.pending_export = None;
                        self.toast = Some(Toast::new("Export cancelled.", 2.5));
                    }
                }
                self.modal.close();
            }

            let empty_prefab_save_opt = EMPTY_PREFAB_SAVE_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = empty_prefab_save_opt {
                if result == EmptyPrefabSaveConfirmResult::Delete {
                    self.confirm_empty_prefab_save_delete();
                }
                self.modal.close();
            }

            let empty_prefab_exit_opt = EMPTY_PREFAB_EXIT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = empty_prefab_exit_opt {
                self.confirm_empty_prefab_transition(result);
                self.modal.close();
            }

            let delete_prefab_opt = DELETE_PREFAB_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = delete_prefab_opt {
                if matches!(result, ConfirmPromptResult::Confirmed) {
                    self.confirm_delete_prefab();
                }
                self.modal.close();
            }

            let dirty_prefab_exit_opt = DIRTY_PREFAB_EXIT_RESULT.with(|c| c.borrow_mut().take());

            if let Some(result) = dirty_prefab_exit_opt {
                self.confirm_dirty_prefab_transition(result);
                self.modal.close();
            }
        }
        None
    }

    /// Returns `true` and creates a toast notification if a prefab with the given name already exists.
    fn duplicate_prefab_name_exists(&mut self, name: &str) -> bool {
        let duplicate_exists = self
            .game
            .prefab_manager
            .prefabs
            .values()
            .any(|prefab| prefab.name == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(
                format!("A prefab named \"{name}\" already exists."),
                2.5,
            ));
        }

        duplicate_exists
    }

    /// Returns `true` and creates a toast notification if another prefab (excluding `exclude_id`) has the given name.
    fn duplicate_prefab_name_exists_excluding(&mut self, name: &str, exclude_id: PrefabId) -> bool {
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

    /// Returns `true` and creates a toast notification if a duplicate game name exists.
    fn duplicate_game_exists(&mut self, name: &String) -> bool {
        let duplicate_exists = list_game_names().iter().any(|existing| existing == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(format!("\"{name}\" already exists."), 2.5));
        };

        duplicate_exists
    }

    pub(crate) fn should_ignore_modal_clicked_outside(&self) -> bool {
        self.prefab_state.require_picker()
    }
}

thread_local! {
    pub static NEW_GAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static RENAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static SAVE_AS_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static PREFAB_NAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static PREFAB_PICKER_RESULT: RefCell<Option<PrefabPickerResult>> = const { RefCell::new(None) };
    pub static WORLD_SETTINGS_RESULT: RefCell<Option<WorldSettingsResult>> = const { RefCell::new(None) };
    pub static EXPORT_OVERWRITE_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
    pub static EMPTY_PREFAB_SAVE_RESULT: RefCell<Option<EmptyPrefabSaveConfirmResult>> = const { RefCell::new(None) };
    pub static EMPTY_PREFAB_EXIT_RESULT: RefCell<Option<EmptyPrefabExitPromptResult>> = const { RefCell::new(None) };
    pub static DIRTY_PREFAB_EXIT_RESULT: RefCell<Option<DirtyPrefabExitPromptResult>> = const { RefCell::new(None) };
    pub static DELETE_PREFAB_RESULT: RefCell<Option<ConfirmPromptResult>> = const { RefCell::new(None) };
}
