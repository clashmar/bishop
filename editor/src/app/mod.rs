// editor/src/editor/mod.rs
mod actions;
pub mod camera_controller;
pub mod sub_editor;

pub use camera_controller::EditorCameraController;
pub use sub_editor::SubEditor;

use crate::canvas::grid_shader::GridRenderer;
use crate::game::game_editor::GameEditor;
use crate::gui::menu_bar::MenuBar;
use crate::gui::modal::Modal;
use crate::menu::MenuEditor;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::*;
use crate::room::room_editor::RoomEditor;
use crate::storage::editor_storage;
use crate::storage::editor_storage::*;
use crate::tilemap::tile_palette::TilePalette;
use crate::with_panel_manager;
use crate::world::world_editor::WorldEditor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::io;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorMode {
    Game,
    World(WorldId),
    Room(RoomId),
    Menu,
}

pub struct Editor {
    pub game: Game,
    pub mode: EditorMode,
    pub return_mode: Option<EditorMode>,
    pub game_editor: GameEditor,
    pub world_editor: WorldEditor,
    pub room_editor: RoomEditor,
    pub menu_editor: MenuEditor,
    pub camera: Camera2D,
    pub cur_world_id: Option<WorldId>,
    pub cur_room_id: Option<RoomId>,
    pub render_system: RenderSystem,
    pub menu_bar: MenuBar,
    pub modal: Modal,
    pub toast: Option<Toast>,
    pub playtest_process: Option<PlaytestProcess>,
    pub grid_renderer: Option<GridRenderer>,
    pub audio_manager: AudioManager,
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
        crate::editor_assets::init_editor_icons(&mut *ctx.borrow_mut());

        // Register all panels
        with_panel_manager(|panel_manager| {
            panel_manager.register_all_panels(&mut *ctx.borrow_mut());
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

        // Initialize the grid renderer
        editor.grid_renderer = Some(GridRenderer::new(&ctx.borrow()));

        Ok(editor)
    }

    pub async fn update(&mut self, ctx: &mut WgpuContext) {
        if let Some(ref mut process) = self.playtest_process {
            if !process.poll() {
                self.playtest_process = None;
            }
        }

        let ui_blocked = self.current_editor().should_block_canvas(ctx);

        if !self.room_editor.view_preview && !ui_blocked {
            EditorCameraController::update(ctx, &mut self.camera);
        }

        match self.mode {
            EditorMode::Menu => {
                self.menu_editor.update(ctx, &self.camera);
            }
            EditorMode::Game => {
                // Returns the id of the world that was clicked on or None
                if let Some(world_id) = self
                    .game_editor
                    .update(ctx, &self.camera, &mut self.game)
                    .await
                {
                    self.world_editor.init_camera(
                        ctx,
                        &mut self.camera,
                        self.game.get_world_mut(world_id),
                    );
                    self.game.current_world_id = world_id;
                    self.cur_world_id = Some(world_id);
                    self.mode = EditorMode::World(world_id);
                }
            }
            EditorMode::World(world_id) => {
                // Returns the id of the room that was clicked on or None
                if let Some(room_id) = self
                    .world_editor
                    .update(ctx, &mut self.camera, &mut self.game)
                    .await
                {
                    self.cur_room_id = Some(room_id);
                    self.mode = EditorMode::Room(room_id);

                    // The world current room must be set
                    self.game.get_world_mut(world_id).current_room_id = Some(room_id);
                }

                // Handle escape
                if Controls::escape(ctx) && !input_is_focused() {
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
                {
                    let current_world = &mut self
                        .game
                        .worlds
                        .iter_mut()
                        .find(|w| w.id == self.game.current_world_id)
                        .expect("Current world id not present in game.");

                    self.room_editor
                        .update(
                            ctx,
                            &mut self.camera,
                            room_id,
                            &mut self.game.ecs,
                            current_world,
                            &mut self.game.asset_manager,
                        )
                        .await;

                    if let Some(msg) = self.room_editor.take_pending_toast() {
                        self.toast = Some(Toast::new(msg, 2.5));
                    }

                    collider_system::update_colliders_from_sprites(
                        &mut self.game.ecs,
                        &mut self.game.asset_manager,
                    );

                    if Controls::escape(ctx) && !input_is_focused() {
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

                        // Save everything
                        self.save();
                    }
                }

                // Launch play‑test if the play button was pressed
                if self.room_editor.request_play {
                    // Write the payload
                    let room = self.get_room_from_id(&room_id);
                    let payload_path = match write_playtest_payload(room, &self.game) {
                        Ok(p) => p,
                        Err(e) => {
                            onscreen_error!("Could not write playtest payload: {e}");
                            return;
                        }
                    };

                    // If in dev mode the binary will be built first
                    match resolve_playtest_binary().await {
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
                    // Reset the request flag so multiple processes don't spawn (and really ruin everything)
                    self.room_editor.request_play = false;
                }
            }
        }

        self.handle_shortcuts(ctx).await;
        self.audio_manager.poll(ctx.get_frame_time());
    }

    pub async fn draw(&mut self, ctx: &mut WgpuContext) {
        match self.mode {
            EditorMode::Menu => self.menu_editor.draw(ctx, &self.camera),
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
                    self.room_editor
                        .draw(
                            ctx,
                            &self.camera,
                            room_id,
                            &mut self.game,
                            &mut self.render_system,
                            grid_renderer,
                        )
                        .await;
                }
            }
        }

        // Draw global UI here
        self.draw_ui(ctx).await;
    }

    async fn draw_ui(&mut self, ctx: &mut WgpuContext) {
        if !self.room_editor.view_preview {
            ctx.set_default_camera();

            // Draw all panels
            with_panel_manager(|panel_manager| {
                panel_manager.update_and_draw(ctx, self.mode, self);
            });

            // Global menu options
            self.draw_menu_bar(ctx).await;

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
            EditorMode::Game => &self.game_editor,
            EditorMode::World(_) => &self.world_editor,
            EditorMode::Room(_) => &self.room_editor,
        }
    }
}
