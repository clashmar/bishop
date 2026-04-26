// editor/src/editor/mod.rs
mod actions;
mod audio;
pub mod camera_controller;
pub(crate) mod escape;
#[cfg(target_os = "macos")]
pub(crate) mod macos_quit;
mod modals;
mod persistence;
mod queries;
pub mod sub_editor;
mod validation;

pub use camera_controller::EditorCameraController;
pub use sub_editor::SubEditor;

use crate::app::audio::default_audio_manager;
use crate::canvas::grid_shader::GridRenderer;
use crate::editor_global::{push_throbbing_toast, push_toast};
use crate::game::game_editor::GameEditor;
use crate::gui::menu_bar::MenuBar;
use crate::gui::modals::delete_world::DeleteWorldModal;
use crate::gui::modals::edit_world::EditWorldModal;
use crate::gui::modals::{Modal, ModalHandler, ModalRegistry};
use crate::gui::modals::delete_prefab::DeletePrefabModal;
use crate::gui::modals::prefab_picker::PrefabPickerModal;
use crate::menu::MenuEditor;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::*;
use crate::prefab::prefab_editor::{PrefabEditor, PrefabStage};
use crate::prefab::PrefabSessionState;
use crate::room::room_editor::{self, RoomEditor};
use crate::storage::editor_storage;
use crate::storage::editor_storage::*;
use crate::storage::export::PendingExport;
use crate::tilemap::tile_palette::TilePalette;
use crate::with_panel_manager;
use crate::world::world_editor::WorldEditor;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::task::BackgroundTask;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorMode {
    Game,
    World(WorldId),
    Room(RoomId),
    Prefab(PrefabId),
    Menu,
}

pub struct Editor {
    pub game: Game,
    pub mode: EditorMode,
    pub return_mode: Option<EditorMode>,
    pub game_editor: GameEditor,
    pub world_editor: WorldEditor,
    pub room_editor: RoomEditor,
    pub prefab_editor: Option<PrefabEditor>,
    pub prefab_stage: Option<PrefabStage>,
    pub menu_editor: MenuEditor,
    pub camera: Camera2D,
    pub cur_world_id: Option<WorldId>,
    pub cur_room_id: Option<RoomId>,
    pub render_system: RenderSystem,
    pub menu_bar: MenuBar,
    pub modal: Modal,
    pub modal_handlers: ModalRegistry,
    pub pending_export: Option<PendingExport>,
    pub(crate) prefab_state: PrefabSessionState,
    pub(crate) pending_camera_reset: bool,
    pub toast: Option<Toast>,
    pub playtest_process: Option<PlaytestProcess>,
    pub pending_playtest_build: Option<BackgroundTask<Result<(PathBuf, PathBuf), String>>>,
    pub grid_renderer: Option<GridRenderer>,
    pub audio_manager: AudioManager,
    pub(crate) last_save_hash: u64,
    pub(crate) handling_close: bool,
}

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
            prefab_editor: None,
            prefab_stage: None,
            menu_editor: MenuEditor::new(),
            cur_world_id: None,
            cur_room_id: None,
            render_system: RenderSystem::with_default_grid_size(),
            menu_bar: MenuBar::new(),
            modal: Modal::default(),
            modal_handlers: ModalRegistry::new(),
            pending_export: None,
            prefab_state: PrefabSessionState::default(),
            pending_camera_reset: false,
            toast: None,
            playtest_process: None,
            pending_playtest_build: None,
            grid_renderer: None,
            audio_manager: default_audio_manager(),
            last_save_hash: 0,
            handling_close: false,
        }
    }
}

impl Editor {
    pub async fn new(ctx: PlatformContext) -> io::Result<Self> {
        let mut editor = Editor::default();

        let game = if let Some(name) = most_recent_game_name() {
            load_game_by_name(&name)?
        } else if let Some(name) = editor.prompt_new_game(ctx.clone()).await {
            create_new_game(name)
        } else {
            // User pressed Cancel
            onscreen_info!("User cancelled new game dialogue.");
            std::process::exit(0);
        };

        // Initialize editor icon textures using the graphics context.
        {
            let ctx_ref = ctx.borrow();
            crate::editor_assets::init_editor_icons(&*ctx_ref);
        }

        // Register all panels
        with_panel_manager(|panel_manager| {
            panel_manager.register_all_panels(&ctx.borrow());
        });

        let palette = match load_palette(&game.name.clone()) {
            Ok(p) => p,
            Err(e) => {
                onscreen_error!("Failed to load palette: {e}");
                // Fall back to a new palette
                TilePalette::new()
            }
        };

        editor.game = editor.init_game_for_editor(&ctx.borrow(), game);

        // Give the palette to the tilemap editor
        editor.room_editor.tilemap_editor.tilemap_panel.palette = palette;
        editor.load_prefab_palette_state();

        // Initialize the grid renderer
        editor.grid_renderer = Some(GridRenderer::new(&ctx.borrow()));

        editor.update_save_state_hash();
        editor.register_modal_handlers();
        Ok(editor)
    }

    pub fn update(&mut self, ctx: &mut WgpuContext) {
        self.update_handle_close_request(ctx);
        if ctx.is_close_requested() && ctx.is_exit_confirmed() {
            return;
        }

        if let Some(ref mut process) = self.playtest_process {
            if !process.poll() {
                self.playtest_process = None;
            }
        }

        if let Some(ref mut build_task) = self.pending_playtest_build {
            if let Some(result) = build_task.poll() {
                self.pending_playtest_build = None;
                push_toast("Playtest ready", 2.0);
                match result {
                    Ok((exe_path, payload_path)) => {
                        if let Some(ref mut old_process) = self.playtest_process {
                            old_process.kill();
                        }
                        match PlaytestProcess::spawn(&exe_path, &payload_path) {
                            Ok(process) => {
                                self.playtest_process = Some(process);
                            }
                            Err(e) => {
                                onscreen_error!("Failed to launch playtest: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        onscreen_error!("Playtest build failed: {e}");
                    }
                }
            }
        }

        escape::resolve_escape(Controls::escape(ctx));

        let ui_blocked = self.current_editor().should_block_canvas(ctx);

        if !self
            .active_room_editor()
            .is_some_and(|editor| editor.view_preview)
            && !ui_blocked
        {
            EditorCameraController::update(ctx, &mut self.camera);
        }

        match self.mode {
            EditorMode::Menu => {
                self.menu_editor.update(ctx, &self.camera);
            }
            EditorMode::Prefab(_) => {
                let open_prefab_picker_requested =
                    if let (Some(prefab_editor), Some(prefab_stage)) =
                        (self.prefab_editor.as_mut(), self.prefab_stage.as_mut())
                    {
                        let mut prefab_ctx = prefab_stage.ctx_mut();
                        prefab_editor.update(ctx, &mut self.camera, &mut prefab_ctx);
                        std::mem::take(&mut prefab_editor.open_prefab_picker_requested)
                    } else {
                        false
                    };
                let delete_prefab_requested = self
                    .prefab_editor
                    .as_mut()
                    .map(|prefab_editor| std::mem::take(&mut prefab_editor.delete_prefab_requested))
                    .unwrap_or(false);

                self.reconcile_active_prefab_room_preview();
                if delete_prefab_requested {
                    DeletePrefabModal.open(self, ctx);
                }
                if open_prefab_picker_requested
                    || (self.prefab_state.require_picker() && !self.modal.is_open())
                {
                    PrefabPickerModal.open(self, ctx);
                }
                if matches!(self.mode, EditorMode::Prefab(_))
                    && escape::escape_available_for_editor()
                    && !input_is_focused()
                {
                    self.request_exit_prefab_mode(ctx);
                }
            }
            EditorMode::Game => {
                // Returns the id of the world that was clicked on or None
                if let Some(world_id) = self.game_editor.update(ctx, &self.camera, &mut self.game) {
                    self.world_editor.init_camera(
                        ctx,
                        &mut self.camera,
                        self.game.get_world_mut(world_id),
                    );
                    self.game.current_world_id = Some(world_id);
                    self.cur_world_id = Some(world_id);
                    self.mode = EditorMode::World(world_id);
                }

                if self.game_editor.pending_edit_world.is_some() {
                    EditWorldModal.open(self, ctx);
                }
                if self.game_editor.pending_delete_world.is_some() {
                    DeleteWorldModal.open(self, ctx);
                }
            }
            EditorMode::World(world_id) => {
                if self.pending_camera_reset {
                    self.pending_camera_reset = false;
                    self.world_editor.init_camera(
                        ctx,
                        &mut self.camera,
                        self.game.get_world_mut(world_id),
                    );
                }
                // Returns the id of the room that was clicked on or None
                if let Some(room_id) =
                    self.world_editor
                        .update(ctx, &mut self.camera, &mut self.game)
                {
                    self.cur_room_id = Some(room_id);
                    self.mode = EditorMode::Room(room_id);

                    // The world current room must be set
                    self.game.get_world_mut(world_id).current_room_id = Some(room_id);

                    // Init camera immediately, as game_editor/world_editor do on their transitions
                    let world = self.game.get_world_mut(world_id);
                    if let Some(room) = world.get_room(room_id) {
                        RoomEditor::init_camera(ctx, &mut self.camera, room, world.grid_size);
                    }
                }

                // Handle escape
                if escape::escape_available_for_editor() && !input_is_focused() {
                    self.game_editor
                        .init_camera(ctx, &mut self.camera, &mut self.game);

                    // Clean up
                    self.cur_world_id = None;
                    self.world_editor.reset();
                    self.mode = EditorMode::Game;

                    // Save everything
                    self.save();
                }
            }
            EditorMode::Room(room_id) => {
                if self.pending_camera_reset {
                    self.pending_camera_reset = false;
                    if let Some((grid_size, room)) = self
                        .game
                        .worlds
                        .iter()
                        .find(|w| Some(w.id) == self.game.current_world_id)
                        .and_then(|world| world.get_room(room_id).map(|r| (world.grid_size, r)))
                    {
                        RoomEditor::init_camera(ctx, &mut self.camera, room, grid_size);
                    }
                }

                let room_prefab_action;
                let mut save_prefab_palette = false;
                let mut save_after_room_exit = false;
                {
                    let active_prefab_stamp = room_editor::ActivePrefabStampState {
                        available: self.room_editor.active_prefab_id.is_some_and(|prefab_id| {
                            self.game.prefab_manager.prefabs.contains_key(&prefab_id)
                        }),
                        pivot: self
                            .room_editor
                            .active_prefab_snap_pivot(&self.game.prefab_manager),
                    };
                    let current_world = &mut self
                        .game
                        .worlds
                        .iter_mut()
                        .find(|w| Some(w.id) == self.game.current_world_id)
                        .expect("Current world id not present in game.");

                    self.room_editor.update(
                        ctx,
                        &mut self.camera,
                        room_editor::RoomEditorUpdateState {
                            room_id,
                            ecs: &mut self.game.ecs,
                            current_world,
                            asset_registry: &mut self.game.asset_registry,
                            sprite_manager: &mut self.game.sprite_manager,
                            active_prefab_stamp,
                        },
                    );
                    room_prefab_action = self.room_editor.prefab_action_request.take();

                    collider_system::update_colliders_from_sprites(
                        &mut self.game.ecs,
                        &mut self.game.sprite_manager,
                    );

                    if escape::escape_available_for_editor()
                        && !input_is_focused()
                        && self.room_editor.reset_scene_sub_mode()
                    {
                        save_prefab_palette = true;
                    } else if escape::escape_available_for_editor() && !input_is_focused() {
                        let palette = &mut self.room_editor.tilemap_editor.tilemap_panel.palette;

                        if let Err(e) = editor_storage::save_palette(palette, &self.game.name) {
                            onscreen_error!("Could not save tile palette: {e}")
                        }

                        // Find the room we just left for center_on_room
                        if let Some(room) = current_world.rooms.iter().find(|m| m.id == room_id) {
                            self.world_editor.center_on_room(
                                ctx,
                                &mut self.camera,
                                room,
                                current_world.grid_size,
                            );
                        }

                        // Clean up
                        self.cur_room_id = None;
                        self.room_editor.reset();
                        self.mode = EditorMode::World(current_world.id);

                        save_prefab_palette = true;
                        save_after_room_exit = true;
                    }
                }

                if save_prefab_palette {
                    self.save_prefab_palette_state();
                }
                if save_after_room_exit {
                    self.save();
                }

                if let Some(request) = room_prefab_action {
                    self.handle_room_prefab_action(ctx, request, room_id);
                }

                // Launch play‑test if the play button was pressed
                if self.room_editor.request_play {
                    if self.pending_playtest_build.is_none() {
                        // Serialize payload synchronously (needs &self.game which isn't Send)
                        let room = self.get_room_from_id(&room_id);
                        let payload_path = match write_playtest_payload(room, &self.game) {
                            Ok(p) => p,
                            Err(e) => {
                                onscreen_error!("Could not write playtest payload: {e}");
                                self.room_editor.request_play = false;
                                return;
                            }
                        };

                        // In release mode, binary extraction is instant — launch synchronously
                        if !cfg!(debug_assertions) {
                            match resolve_playtest_binary() {
                                Ok(exe_path) => {
                                    if let Some(ref mut old_process) = self.playtest_process {
                                        old_process.kill();
                                    }
                                    match PlaytestProcess::spawn(&exe_path, &payload_path) {
                                        Ok(process) => {
                                            self.playtest_process = Some(process);
                                        }
                                        Err(e) => {
                                            onscreen_error!("Failed to launch playtest: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    onscreen_error!("{e}");
                                }
                            }
                        } else {
                            // Dev mode: cargo build runs in the background
                            push_throbbing_toast("Building playtest");
                            self.pending_playtest_build = Some(BackgroundTask::spawn(move || {
                                resolve_playtest_binary()
                                    .map(|exe_path| (exe_path, payload_path))
                                    .map_err(|e| e.to_string())
                            }));
                        }
                    }
                    self.room_editor.request_play = false;
                }
            }
        }

        self.handle_shortcuts(ctx);
        self.audio_manager.poll(ctx.get_frame_time());
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) {
        if self.pending_camera_reset {
            self.pending_camera_reset = false;
            self.game_editor
                .init_camera(ctx, &mut self.camera, &mut self.game);
        }

        match self.mode {
            EditorMode::Menu => self.menu_editor.draw(ctx, &self.camera),
            EditorMode::Prefab(_) => {
                if let (Some(prefab_editor), Some(prefab_stage), Some(grid_renderer)) = (
                    self.prefab_editor.as_mut(),
                    self.prefab_stage.as_mut(),
                    &self.grid_renderer,
                ) {
                    let mut prefab_ctx = prefab_stage.ctx_mut();
                    prefab_editor.draw(ctx, &self.camera, &mut prefab_ctx, grid_renderer);
                }
            }
            EditorMode::Game => {
                self.game_editor.draw(ctx, &mut self.camera, &mut self.game);
            }
            EditorMode::World(world_id) => {
                // World id should already be set
                if self.cur_world_id.is_none() {
                    self.cur_world_id = Some(world_id);
                }

                if let Some(grid_renderer) = &self.grid_renderer {
                    self.world_editor.draw(
                        ctx,
                        world_id,
                        &self.camera,
                        &mut self.game,
                        grid_renderer,
                    );
                }
            }
            EditorMode::Room(room_id) => {
                // Room id should already be set
                if self.cur_room_id.is_none() {
                    self.cur_room_id = Some(room_id);
                }

                if let Some(grid_renderer) = &self.grid_renderer {
                    self.room_editor.draw(
                        ctx,
                        &self.camera,
                        room_id,
                        &mut self.game,
                        &mut self.render_system,
                        grid_renderer,
                    );
                }
            }
        }

        // Draw global UI here
        self.draw_ui(ctx);
    }

    fn draw_ui(&mut self, ctx: &mut WgpuContext) {
        if !self
            .active_room_editor()
            .is_some_and(|editor| editor.view_preview)
        {
            ctx.set_default_camera();

            // Draw all panels
            with_panel_manager(|panel_manager| {
                panel_manager.update_and_draw(ctx, self.mode, self);
            });

            // Global menu options
            self.draw_menu_bar(ctx);

            // Draws and handles result of modal
            if self.handle_modal(ctx).is_some() {
                self.modal.close();
            }

            self.draw_toast(ctx);
        }
    }

    fn current_editor(&self) -> &dyn SubEditor {
        match self.mode {
            EditorMode::Menu => &self.menu_editor,
            EditorMode::Prefab(_) => self
                .prefab_editor
                .as_ref()
                .map(|editor| editor as &dyn SubEditor)
                .unwrap_or(&self.room_editor),
            EditorMode::Game => &self.game_editor,
            EditorMode::World(_) => &self.world_editor,
            EditorMode::Room(_) => &self.room_editor,
        }
    }

    pub fn current_mode(&self) -> EditorMode {
        self.mode
    }

    pub fn active_room_editor(&self) -> Option<&RoomEditor> {
        match self.mode {
            EditorMode::Room(_) => Some(&self.room_editor),
            _ => None,
        }
    }
}
