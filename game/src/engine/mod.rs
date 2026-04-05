// game/src/engine/mod.rs
// Keep `mod.rs` limited to frame orchestration. Feature-specific methods belong in focused
// helper modules alongside the subsystem it serves, or in a new engine sub-module.
mod audio_events;
pub mod engine_builder;
pub mod game_instance;
mod render;
#[cfg(test)]
mod tests;
use audio_events::emit_pending_audio_events;
use render::*;

pub use engine_builder::EngineBuilder;
pub use game_instance::GameInstance;

use crate::diagnostics::DiagnosticsOverlay;
use crate::game_global::set_menu_active;
use crate::physics::physics_system::*;
use crate::scripting::script_system::ScriptSystem;
use crate::transitions::transition_manager::TransitionManager;
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::prelude::*;
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Engine {
    /// Currently running instance of the game.
    pub game_instance: Rc<RefCell<GameInstance>>,
    /// Current state of the active game.
    pub game_state: GameState,
    /// Platform context for input/rendering.
    pub ctx: PlatformContext,
    /// Single Lua VM.
    pub lua: Lua,
    /// Camera manager for the game.
    pub camera_manager: CameraManager,
    /// Rendering system for the game.
    pub render_system: RenderSystem,
    /// Runtime diagnostics overlay (playtest only).
    pub diagnostics: DiagnosticsOverlay,
    /// Menu system for pause and overlay menus.
    pub menu_manager: MenuManager,
    /// Whether the engine is running in playtest mode.
    pub is_playtest: bool,
    /// Accumulator for fixed timestep updates.
    pub accumulator: f32,
    /// Exponential moving average of frame time, used to smooth accumulator input.
    pub smoothed_dt: Option<f32>,
    /// Background audio service, polled once per frame.
    pub audio_manager: AudioManager,
}

/// Represents the current state of the active game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    /// A front-end root menu is open and gameplay is frozen.
    StartMenu,
    /// Normal gameplay is running.
    Playing,
    /// A gameplay pause menu is open and gameplay is frozen.
    Paused,
}

/// Configures how a loaded session enters the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEntryMode {
    /// Open the given root menu and hold gameplay in the start-menu state.
    StartMenu { menu_id: String },
    /// Start the session in gameplay.
    Playing,
}

impl BishopApp for Engine {
    async fn frame(&mut self, ctx: PlatformContext) {
        let raw_dt = ctx.borrow().get_frame_time();
        let smoothed = smooth_dt(&mut self.smoothed_dt, raw_dt, 0.9);
        let dt = snap_dt(smoothed);

        self.update_game_state();

        self.menu_manager.handle_input(&mut *ctx.borrow_mut());
        emit_pending_audio_events(self);

        if self.is_playtest {
            self.diagnostics.update(raw_dt);
            self.diagnostics.handle_input(&mut *ctx.borrow_mut());
        }

        if self.game_state == GameState::Playing {
            self.accumulator = (self.accumulator + dt).min(MAX_ACCUM);

            while self.accumulator >= FIXED_DT {
                self.accumulator -= FIXED_DT;
                self.fixed_update(&mut *ctx.borrow_mut(), FIXED_DT);
            }

            self.update(raw_dt);
        }

        // Drain audio commands pushed by scripts this frame
        self.audio_manager.poll(raw_dt);

        if self.is_playtest {
            self.diagnostics.update_from_game(
                &self.game_instance.borrow(),
                self.render_system.render_time_ms,
                &self.audio_manager,
            );
        }

        let alpha = (self.accumulator / FIXED_DT).clamp(0.0, 1.0);
        self.render(&ctx, alpha);

        // Process ui events and emit to Lua
        self.game_instance.borrow().drain_ui_events();
    }
}

impl Engine {
    /// Creates a new Engine with the given configuration and session entry mode.
    pub fn new(
        game_instance: Rc<RefCell<GameInstance>>,
        ctx: PlatformContext,
        lua: Lua,
        camera_manager: CameraManager,
        grid_size: f32,
        is_playtest: bool,
        entry_mode: EngineEntryMode,
    ) -> Self {
        let mut menu_manager = MenuManager::new();
        menu_manager.load_templates_from_disk();
        menu_manager.set_action_handler(GameMenuHandler);

        let game_state = apply_entry_mode(&mut menu_manager, entry_mode);

        Self {
            game_instance,
            game_state,
            ctx,
            lua,
            camera_manager,
            render_system: RenderSystem::with_grid_size(grid_size),
            diagnostics: DiagnosticsOverlay::new(),
            menu_manager,
            is_playtest,
            accumulator: 0.0,
            smoothed_dt: None,
            audio_manager: AudioManager::new::<PlatformAudioBackend>(),
        }
    }

    pub fn fixed_update<C: BishopContext>(&mut self, ctx: &mut C, dt: f32) {
        let mut game_instance = self.game_instance.borrow_mut();
        game_instance.store_previous_positions(&mut self.camera_manager);

        {
            let game_ctx = game_instance.game.ctx_mut();
            let Some(cur_world) = game_ctx.cur_world.as_deref() else {
                return;
            };
            let Some(current_room) = cur_world.current_room() else {
                return;
            };
            update_physics(
                game_ctx.asset_manager,
                game_ctx.ecs,
                current_room,
                dt,
                cur_world.grid_size,
            );
        }

        // Resolve room transitions before updating the camera
        TransitionManager::handle_transitions(&mut game_instance);

        let game_ctx = game_instance.game.ctx_mut();
        if let Some(cur_world) = game_ctx.cur_world.as_deref() {
            if let Some(current_room) = cur_world.current_room() {
                self.camera_manager.update_active(
                    ctx,
                    game_ctx.ecs,
                    current_room,
                    cur_world.grid_size,
                );
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        {
            // Keep borrow_mut in this scope
            let mut game_instance = self.game_instance.borrow_mut();
            update_speech_timers(&mut game_instance.game.ecs, dt);

            let game_ctx = game_instance.game.ctx_mut();
            let asset_manager = game_ctx.asset_manager;
            let ecs = game_ctx.ecs;

            if let Some(cur_world) = game_ctx.cur_world.as_deref() {
                if let Some(current_room) = cur_world.current_room() {
                    let loader = self.ctx.borrow();
                    update_animation_sytem(&*loader, ecs, asset_manager, dt, current_room.id);
                }
            }

            // Load scripts in this scope TODO: make this part of run_scripts when scope is finalized
            let ctx = game_instance.game.ctx_mut();
            if let Err(e) = ScriptSystem::load_scripts(&self.lua, ctx.ecs, ctx.script_manager) {
                onscreen_error!("Error loading scripts: {}", e);
            }
        }

        // Sync menu state for Lua scripts
        set_menu_active(self.menu_manager.has_active_menu());

        // Run scripts outside borrow_mut scope
        if let Err(e) = ScriptSystem::run_scripts(dt, self) {
            onscreen_error!("Error running scripts: {}", e);
        }
    }

    pub fn render(&mut self, ctx: &PlatformContext, alpha: f32) {
        if !self.menu_manager.is_hiding_game() {
            let mut ctx_borrow = ctx.borrow_mut();
            let platform_ctx = &mut *ctx_borrow;
            let render_cam = build_render_camera(&self.camera_manager, alpha);
            let mut game_borrow = self.game_instance.borrow_mut();
            let game_instance = &mut *game_borrow;

            render_scene(
                platform_ctx,
                game_instance,
                &mut self.render_system,
                &render_cam,
                alpha,
            );

            render_screen_space(platform_ctx, game_instance, &render_cam, alpha);

            if self.is_playtest {
                self.diagnostics.draw(platform_ctx);
            }
        } else {
            ctx.borrow_mut().clear_background(Color::BLACK);
        }

        self.render_menus(ctx);
    }

    /// Resolves the current game state from all active systems.
    fn update_game_state(&mut self) {
        self.game_state = resolve_game_state(self.game_state.clone(), &self.menu_manager);
    }
}

fn apply_entry_mode(menu_manager: &mut MenuManager, entry_mode: EngineEntryMode) -> GameState {
    match entry_mode {
        EngineEntryMode::StartMenu { menu_id } => {
            menu_manager.set_input_policy(MenuInputPolicy::FrontEnd);
            menu_manager.open_menu(&menu_id);
            GameState::StartMenu
        }
        EngineEntryMode::Playing => GameState::Playing,
    }
}

fn resolve_game_state(current_state: GameState, menu_manager: &MenuManager) -> GameState {
    if matches!(current_state, GameState::StartMenu) && menu_manager.has_active_menu() {
        return GameState::StartMenu;
    }

    if menu_manager.is_pausing_game() {
        GameState::Paused
    } else {
        GameState::Playing
    }
}
