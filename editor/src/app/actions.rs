// editor/src/editor/actions.rs
use crate::app::*;
use crate::commands::scene::DeletePrefabCmd;
use crate::editor_global::*;
use crate::gui::inspector::audio_source_module::clear_active_audio_preview;
use crate::gui::menu_bar::*;
use crate::gui::panels::*;
use crate::prefab::{PendingPrefabTransition, PrefabTransitionPrompt};
use crate::storage::editor_storage::*;
use bishop::prelude::*;
use engine_core::prelude::*;

impl Editor {
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
            EditorMode::Prefab(_) => self
                .prefab_editor
                .as_ref()
                .map(|editor| editor.prefab_name.clone())
                .unwrap_or_else(|| "Prefab".to_string()),
            EditorMode::Menu => "Menu Editor".to_string(),
        };

        if let Some(action) = self.menu_bar.draw(ctx, &menu_title, self.mode) {
            self.run_action(ctx, action);
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &mut WgpuContext) {
        if let Some(action) = self.shortcut_action(ctx) {
            self.run_action(ctx, action);
        }
    }

    fn shortcut_action(&self, ctx: &WgpuContext) -> Option<EditorAction> {
        let input_focused = input_is_focused();
        let actions = [
            EditorAction::Save,
            EditorAction::SaveAs,
            EditorAction::Undo,
            EditorAction::Redo,
            EditorAction::ViewConsolePanel,
            EditorAction::ViewDiagnosticsPanel,
            EditorAction::ViewHierarchyPanel,
            EditorAction::ViewPrefabBrowserPanel,
            EditorAction::ViewPrefabPalettePanel,
            EditorAction::ViewResourcesPanel,
        ];

        actions.into_iter().find(|action| {
            action.is_available_in(self.mode)
                && (!input_focused || !action.blocked_by_focused_input())
                && action.shortcut_pressed(ctx)
        })
    }

    fn run_action(&mut self, ctx: &mut WgpuContext, action: EditorAction) {
        match action {
            EditorAction::Rename => {
                self.open_rename_modal(ctx);
            }
            EditorAction::NewGame => {
                self.save();
                self.open_new_game_modal(ctx);
            }
            EditorAction::Open => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use rfd::FileDialog;
                    if let Some(path) = FileDialog::new()
                        .set_directory(absolute_save_root())
                        .pick_folder()
                    {
                        match ensure_inside_save_root(&path) {
                            Ok(_) => {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    match load_game_by_name(name) {
                                        Ok(game) => {
                                            self.reset(ctx, game);
                                            self.toast =
                                                Some(Toast::new(format!("Loaded '{}'", name), 2.5));
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
            EditorAction::Save => {
                if matches!(self.mode, EditorMode::Prefab(_)) {
                    self.request_prefab_save(ctx);
                } else {
                    self.save();
                }
            }
            EditorAction::SaveAs => self.open_save_as_modal(ctx),
            EditorAction::Undo => crate::editor_global::request_undo(),
            EditorAction::Redo => crate::editor_global::request_redo(),
            EditorAction::Export => self.begin_export(ctx),
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
            EditorAction::ViewPrefabBrowserPanel => {
                with_panel_manager(|panel_manager| {
                    panel_manager.toggle(PREFAB_BROWSER_PANEL);
                });
            }
            EditorAction::ViewPrefabPalettePanel => {
                with_panel_manager(|panel_manager| {
                    panel_manager.toggle(PREFAB_PALETTE_PANEL);
                });
            }
            EditorAction::ViewResourcesPanel => {
                with_panel_manager(|panel_manager| {
                    panel_manager.toggle(RESOURCES_PANEL);
                });
            }
            EditorAction::WorldSettings => {
                self.open_world_settings_modal(ctx);
            }
            EditorAction::OpenPrefabEditor => {
                self.open_prefab_editor(ctx);
            }
            EditorAction::OpenMenuEditor => {
                clear_active_audio_preview();
                self.room_editor.reset_scene_sub_mode();
                self.return_mode = Some(self.mode);
                self.mode = EditorMode::Menu;
                self.load_menus();
                self.menu_editor.init_camera(ctx, &mut self.camera);
            }
            EditorAction::ReturnToGameEditor => {
                match self.mode {
                    EditorMode::Menu => {
                        self.save_menus();
                    }
                    EditorMode::Prefab(_) => {
                        self.request_exit_prefab_mode(ctx);
                        return;
                    }
                    _ => {}
                }

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
                    EditorMode::Prefab(_) => {}
                    EditorMode::Menu => {}
                }
            }
        }
    }

    pub fn get_room_from_id(&self, room_id: &RoomId) -> &Room {
        self.game
            .current_world()
            .rooms
            .iter()
            .find(|m| m.id == *room_id)
            .expect("Could not find room from id.")
    }

    pub(crate) fn request_prefab_save(&mut self, ctx: &WgpuContext) {
        if self.is_blank_prefab_mode() {
            self.toast = Some(Toast::new("Blank prefab sessions cannot be saved.", 2.5));
            return;
        }

        let Some(staged_state) = self.active_prefab_staged_state() else {
            return;
        };

        match staged_state {
            crate::prefab::prefab_editor::StagedPrefabState::PrefabAsset(prefab) => {
                self.commit_prefab_asset_save(prefab);
            }
            crate::prefab::prefab_editor::StagedPrefabState::Empty => {
                if !self.active_prefab_is_clean() {
                    self.open_empty_prefab_save_modal(ctx);
                }
            }
        }
    }

    pub(crate) fn request_exit_prefab_mode(&mut self, ctx: &WgpuContext) {
        match self.request_prefab_transition(PendingPrefabTransition::Exit) {
            PrefabTransitionPrompt::None => {}
            PrefabTransitionPrompt::Dirty => self.open_dirty_prefab_exit_modal(ctx),
            PrefabTransitionPrompt::Empty => self.open_empty_prefab_exit_modal(ctx),
        }
    }

    pub(crate) fn confirm_delete_prefab(&mut self) {
        let Some(prefab_id) = self.active_persisted_prefab_id() else {
            return;
        };

        push_command(Box::new(DeletePrefabCmd::new(
            prefab_id,
            EditorMode::Prefab(prefab_id),
        )));
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
        self.load_prefab_palette_state();

        // Render system always needs a resize after switch
        let cur_screen = (ctx.screen_width() as u32, ctx.screen_height() as u32);
        self.render_system.resize(cur_screen.0, cur_screen.1)
    }

    // Returns an initialized game for the editor.
    pub fn init_game_for_editor(&mut self, ctx: &WgpuContext, game: Game) -> Game {
        let mut game = game;

        with_lua(|lua| {
            game.initialize(ctx, lua);
            if let Err(error) = register_runtime_modules(lua, &game.script_manager.event_bus) {
                onscreen_error!("Lua module registration failed: {error}");
            }
        });
        self.game_editor
            .init_camera(ctx, &mut self.camera, &mut game);

        game
    }
}
