// editor/src/editor/actions.rs
use crate::app::Editor;
use crate::app::EditorCameraController;
use crate::app::*;
use crate::commands::game::*;
use crate::commands::world::*;
use crate::editor_global::*;
use crate::game::game_editor::GameEditor;
use crate::gui::inspector::audio_source_module::clear_active_audio_preview;
use crate::gui::menu_bar::*;
use crate::gui::modal::*;
use crate::gui::panels::*;
use crate::gui::prompts::*;
use crate::menu::MenuEditor;
use crate::room::room_editor::RoomEditor;
use crate::storage::editor_storage::*;
use crate::storage::export::export_game;
use crate::world::world_editor::WorldEditor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;

impl Default for Editor {
    fn default() -> Self {
        Self {
            game: Game::default(),
            camera: Camera2D::default(),
            mode: EditorMode::Game,
            return_mode: None,
            game_editor: GameEditor::new(),
            world_editor: WorldEditor::new(),
            room_editor: RoomEditor::new(),
            menu_editor: MenuEditor::new(),
            cur_world_id: None,
            cur_room_id: None,
            render_system: RenderSystem::with_default_grid_size(),
            menu_bar: MenuBar::new(),
            modal: Modal::default(),
            toast: None,
            playtest_process: None,
            grid_renderer: None,
            audio_manager: AudioManager::new::<PlatformAudioBackend>(),
        }
    }
}

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

    pub fn draw_menu_bar(&mut self, ctx: &mut WgpuContext) {
        let menu_title = match self.mode {
            EditorMode::Game => self.game.name.clone(),
            EditorMode::World(_) => self.game.current_world().name.clone(),
            EditorMode::Room(id) => self
                .game
                .current_world()
                .get_room(id)
                .map(|room| room.name.clone())
                .unwrap_or_else(|| "Room".to_string()),
            EditorMode::Menu => "Menu Editor".to_string(),
        };

        if let Some(action) = self.menu_bar.draw(ctx, &menu_title, self.mode) {
            match action {
                EditorAction::Rename => {
                    self.open_rename_modal(ctx);
                }
                EditorAction::NewGame => {
                    // Save current
                    self.save();
                    self.open_new_game_modal(ctx);
                }
                EditorAction::Open => {
                    // Open a folder picker rooted at the absolute save folder
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        use rfd::FileDialog;
                        if let Some(path) = FileDialog::new()
                            .set_directory(absolute_save_root())
                            .pick_folder()
                        {
                            match ensure_inside_save_root(&path) {
                                Ok(_) => {
                                    // Only load if it's in the correct folder
                                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                        match load_game_by_name(name) {
                                            Ok(game) => {
                                                self.reset(ctx, game);
                                                self.toast = Some(Toast::new(
                                                    format!("Loaded '{}'", name),
                                                    2.5,
                                                ));
                                            }
                                            Err(e) => {
                                                onscreen_error!("Failed to load game: {e}");
                                                self.toast = Some(Toast::new(
                                                    "Could not load selected game.",
                                                    2.5,
                                                ));
                                            }
                                        }
                                    } else {
                                        self.toast =
                                            Some(Toast::new("Folder name could not be read.", 2.5));
                                    }
                                }
                                Err(err_msg) => {
                                    self.toast = Some(Toast::new(&err_msg, 3.0));
                                }
                            }
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.toast = Some(Toast::new("Folder picker unavailable in WASM", 2.5));
                    }
                }
                EditorAction::Save => self.save(),
                EditorAction::SaveAs => self.open_save_as_modal(ctx),
                EditorAction::Undo => crate::editor_global::request_undo(),
                EditorAction::Redo => crate::editor_global::request_redo(),
                EditorAction::Export => match export_game(&self.game) {
                    Ok(path) => {
                        self.toast =
                            Some(Toast::new(format!("Exported to: {}", path.display()), 2.5));
                    }
                    Err(e) => {
                        onscreen_error!("Export failed: {e}");
                    }
                },
                EditorAction::ChangeSaveRoot => match change_save_root() {
                    SaveRootResult::Changed(new_root) => {
                        self.toast = Some(Toast::new(
                            format!("Save root moved to: {}", new_root.display()),
                            2.5,
                        ));
                    }
                    SaveRootResult::Cancelled => {}
                    SaveRootResult::Failed => {
                        self.toast = Some(Toast::new("Failed to update save root.", 2.0));
                    }
                },
                EditorAction::ViewHierarchyPanel => {
                    with_panel_manager(|panel_manager| {
                        panel_manager.toggle(HIERARCHY_PANEL);
                    });
                }
                EditorAction::ViewConsolePanel => {
                    with_panel_manager(|panel_manager| {
                        panel_manager.toggle(CONSOLE_PANEL);
                    });
                }
                EditorAction::ViewDiagnosticsPanel => {
                    with_panel_manager(|panel_manager| {
                        panel_manager.toggle(DIAGNOSTICS_PANEL);
                    });
                }
                EditorAction::WorldSettings => {
                    self.open_world_settings_modal(ctx);
                }
                EditorAction::OpenMenuEditor => {
                    clear_active_audio_preview();
                    self.return_mode = Some(self.mode);
                    self.mode = EditorMode::Menu;
                    self.load_menus();
                    MenuEditor::init_camera(ctx, &mut self.camera);
                }
                EditorAction::ReturnToGameEditor => {
                    // Save menus before leaving menu mode
                    self.save_menus();

                    let return_mode = self.return_mode.unwrap_or(EditorMode::Game);
                    self.mode = return_mode;
                    self.return_mode = None;

                    match return_mode {
                        EditorMode::Game => {
                            self.game_editor
                                .init_camera(ctx, &mut self.camera, &mut self.game);
                        }
                        EditorMode::World(id) => {
                            self.world_editor.init_camera(
                                ctx,
                                &mut self.camera,
                                self.game.get_world_mut(id),
                            );
                        }
                        EditorMode::Room(id) => {
                            let current_world = self.game.current_world();
                            if let Some(room) = current_world.get_room(id) {
                                EditorCameraController::reset_room_editor_camera(
                                    ctx,
                                    &mut self.camera,
                                    room,
                                    current_world.grid_size,
                                );
                            }
                        }
                        EditorMode::Menu => {
                            // Should not return to Menu mode
                        }
                    }
                }
            }
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &mut WgpuContext) {
        if Controls::save(ctx) {
            self.save();
        }

        if Controls::save_as(ctx) {
            self.open_save_as_modal(ctx);
        }

        if Controls::undo(ctx) {
            request_undo();
        }

        if Controls::redo(ctx) {
            request_redo();
        }

        if Controls::c(ctx) && !input_is_focused() {
            with_panel_manager(|pm| pm.toggle(CONSOLE_PANEL));
        }

        if Controls::f3(ctx) && !input_is_focused() {
            with_panel_manager(|pm| pm.toggle(DIAGNOSTICS_PANEL));
        }
    }

    pub fn save(&mut self) {
        let palette = &self.room_editor.tilemap_editor.tilemap_panel.palette;
        let palette_saved = if let Err(e) = save_palette(palette, &self.game.name) {
            onscreen_error!("Could not save palette: {e}");
            false
        } else {
            true
        };

        if let Err(e) = save_game(&self.game) {
            onscreen_error!("Could not save game: {}.", e)
        } else if palette_saved {
            self.save_menus();
            self.toast = Some(Toast::new("Saved", 2.5));
        }
    }

    /// Saves all menu templates to disk.
    pub fn save_menus(&self) {
        for template in &self.menu_editor.templates {
            if let Err(e) = save_menu(template) {
                onscreen_error!("Could not save menu '{}': {}", template.id, e);
            }
        }
    }

    /// Loads all menu templates from disk.
    pub fn load_menus(&mut self) {
        let templates = load_menus();
        self.menu_editor.set_templates(templates);
    }

    pub fn get_room_from_id(&self, room_id: &RoomId) -> &Room {
        self.game
            .current_world()
            .rooms
            .iter()
            .find(|m| m.id == *room_id)
            .expect("Could not find room from id.")
    }

    fn open_new_game_modal(&mut self, ctx: &mut WgpuContext) {
        let prompt_message = "Enter game name:";
        let mut prompt = self.set_prompt_modal(ctx, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _asset_manager| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                NEW_GAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    fn open_rename_modal(&mut self, ctx: &mut WgpuContext) {
        let prompt_message = match self.mode {
            EditorMode::Game => "Rename game: ",
            EditorMode::World(_) => "Rename world: ",
            EditorMode::Room(_) => "Rename room: ",
            EditorMode::Menu => "Rename menu: ",
        };

        let mut prompt = self.set_prompt_modal(ctx, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                RENAME_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    fn open_save_as_modal(&mut self, ctx: &mut WgpuContext) {
        let prompt_message = "Save as:";
        let mut prompt = self.set_prompt_modal(ctx, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                SAVE_AS_PROMPT_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    fn set_prompt_modal(&mut self, ctx: &mut WgpuContext, prompt_message: &str) -> StringPrompt {
        self.modal = Modal::new(ctx, 400.0, 180.0);
        StringPrompt::new(self.modal.rect, prompt_message)
    }

    fn open_world_settings_modal(&mut self, ctx: &mut WgpuContext) {
        self.modal = Modal::new(ctx, 300.0, 150.0);
        let world = self.game.current_world();
        let world_id = world.id;
        let grid_size = world.grid_size;

        let mut prompt =
            WorldSettingsPrompt::new(world_id, self.modal.rect, WidgetId::default(), grid_size);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _| {
            if let Some(result) = prompt.draw(ctx) {
                WORLD_SETTINGS_RESULT.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        self.modal.open(widgets);
    }

    pub fn handle_modal(&mut self, ctx: &mut WgpuContext) -> Option<ModalResult> {
        if self.modal.is_open() {
            // Outside‑click handling
            if self.modal.draw(ctx, &mut self.game.asset_manager) {
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
        }
        None
    }

    /// Updates and draws the toast to the screen.
    pub fn draw_toast(&mut self, ctx: &mut WgpuContext) {
        if let Some(pending) = take_pending_toast() {
            self.toast = Some(pending);
        }

        if let Some(toast) = &mut self.toast {
            toast.update(ctx);
            if !toast.active {
                self.toast = None;
            }
        }
    }

    pub fn reset(&mut self, ctx: &WgpuContext, game: Game) {
        // Update global game name for file system
        set_game_name(game.name.clone());

        // Resets the global services (command queue, clipboard etc)
        reset_services();

        let game = self.init_game_for_editor(ctx, game);

        *self = Self {
            game,
            camera: std::mem::take(&mut self.camera),
            ..Self::default()
        };

        // Render system always needs a resize after switch
        let cur_screen = (ctx.screen_width() as u32, ctx.screen_height() as u32);
        self.render_system.resize(cur_screen.0, cur_screen.1)
    }

    // Returns an initialized game for the editor.
    pub fn init_game_for_editor(&mut self, ctx: &WgpuContext, game: Game) -> Game {
        let mut game = game;

        with_lua(|lua| game.initialize(ctx, lua));
        self.game_editor
            .init_camera(ctx, &mut self.camera, &mut game);

        game
    }

    /// Returns `true` and creates a toast notification if a duplicate game name exists.
    fn duplicate_game_exists(&mut self, name: &String) -> bool {
        let duplicate_exists = list_game_names().iter().any(|existing| existing == name);

        if duplicate_exists {
            self.toast = Some(Toast::new(format!("\"{name}\" already exists."), 2.5));
        };

        duplicate_exists
    }
}

thread_local! {
    pub static NEW_GAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static RENAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static SAVE_AS_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
    pub static WORLD_SETTINGS_RESULT: RefCell<Option<WorldSettingsResult>> = const { RefCell::new(None) };
}
