// Keep `mod.rs` limited to frame orchestration. Feature-specific methods belong in focused
// helper modules alongside the subsystem it serves, or in a new engine sub-module.
mod audio_events;
pub mod engine_builder;
pub mod game_instance;
mod render;
pub mod save_runtime;
#[cfg(test)]
mod tests;
use audio_events::emit_pending_audio_events;
use render::*;

pub use engine_builder::EngineBuilder;
pub use game_instance::{GameInstance, PreparedGameInstance};

pub use save_runtime::{RuntimeLoadRequest, SaveRuntime};

use crate::diagnostics::{DiagnosticsOverlay, TimingTraceSample};
use crate::game_global::{set_menu_active, take_pending_world_transition};
use crate::physics::physics_system::*;
use crate::scripting::script_system::ScriptSystem;
use crate::transitions::room_transition_manager::RoomTransitionManager;
use crate::transitions::traversal_residency;
use crate::transitions::world_exit_manager::WorldExitManager;
use crate::transitions::world_transitions::WorldTransitionManager;
use bishop::prelude::*;
use bishop::BishopApp;
use engine_core::animation::{update_animation_sytem};
use engine_core::audio::{AudioManager};
use engine_core::camera::CameraManager;
use engine_core::constants::timing;
use engine_core::diagnostics::TraversalResidencyDiagnostics;
use engine_core::logging::{omni_error};
use engine_core::menu::{GameMenuHandler, MenuInputPolicy, MenuManager, MenuSessionAction};
use engine_core::rendering::{RenderSystem, smooth_dt, snap_dt};
use engine_core::task::BackgroundService;
use engine_core::text::update_speech_timers;
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
    /// Runtime save/restore subsystem.
    pub save_runtime: SaveRuntime,
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
    /// Whether a pause-menu quit should reboot to the title instead of ending the session.
    quit_to_title_enabled: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestedSessionAction {
    QuitToTitle,
    CloseApp,
}

impl BishopApp for Engine {
    async fn frame(&mut self, ctx: PlatformContext) {
        let raw_dt = ctx.borrow().get_frame_time();
        let smoothed = smooth_dt(&mut self.smoothed_dt, raw_dt, 0.9);
        let dt = snap_dt(smoothed);
        let mut fixed_steps = 0_u8;
        let timing_sample = self.is_playtest.then(|| {
            let ctx = ctx.borrow();
            TimingTraceSample::new(raw_dt, dt, &*ctx)
        });

        let gameplay_viewport = gameplay_viewport(ctx.borrow().screen_width(), ctx.borrow().screen_height());
        self.menu_manager.set_viewport(gameplay_viewport);

        self.update_game_state();

        if self.process_menu_input(&ctx) {
            return;
        }
        emit_pending_audio_events(self);

        if let Some(sample) = timing_sample {
            self.diagnostics.update(sample);
            self.diagnostics.handle_input(&mut *ctx.borrow_mut());
        }

        if self.game_state == GameState::Playing {
            self.accumulator = (self.accumulator + dt).min(timing::MAX_ACCUM);

            while self.accumulator >= timing::FIXED_DT {
                self.accumulator -= timing::FIXED_DT;
                fixed_steps = fixed_steps.saturating_add(1);
                self.fixed_update(&mut *ctx.borrow_mut(), timing::FIXED_DT);
            }

            self.update(raw_dt);
            self.apply_pending_world_transition();
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

        let alpha = (self.accumulator / timing::FIXED_DT).clamp(0.0, 1.0);
        if let Some(sample) = timing_sample {
            self.diagnostics.record_timing_trace(
                sample.with_frame_state(fixed_steps, self.accumulator, alpha),
            );
        }
        self.render(&ctx, alpha);

        // Process ui events and any queued menu-open callbacks.
        self.game_instance.borrow().drain_ui_events();
        ScriptSystem::process_commands(self);
    }
}

/// Bundled runtime configuration assembled by EngineBuilder.
pub(crate) struct EngineRuntimeConfig {
    pub(crate) save_runtime: SaveRuntime,
    pub(crate) camera_manager: CameraManager,
    pub(crate) is_playtest: bool,
    pub(crate) quit_to_title_enabled: bool,
    pub(crate) entry_mode: EngineEntryMode,
}

impl Engine {
    /// Creates a new Engine with the given configuration and session entry mode.
    fn new(
        game_instance: Rc<RefCell<GameInstance>>,
        ctx: PlatformContext,
        lua: Lua,
        cfg: EngineRuntimeConfig,
        render_system: RenderSystem,
    ) -> Self {
        let mut menu_manager = MenuManager::new();
        menu_manager.load_templates_from_disk();
        menu_manager.set_action_handler(GameMenuHandler);

        let game_state = apply_entry_mode(&mut menu_manager, cfg.entry_mode);

        let mut engine = Self {
            game_instance,
            game_state,
            ctx,
            lua,
            save_runtime: cfg.save_runtime,
            camera_manager: cfg.camera_manager,
            render_system,
            diagnostics: DiagnosticsOverlay::new(),
            menu_manager,
            is_playtest: cfg.is_playtest,
            quit_to_title_enabled: cfg.quit_to_title_enabled,
            accumulator: 0.0,
            smoothed_dt: None,
            audio_manager: AudioManager::new::<PlatformAudioBackend>(),
        };

        {
            let mut game_instance = engine.game_instance.borrow_mut();
            if engine.is_playtest {
                game_instance.traversal_residency_diagnostics =
                    Some(TraversalResidencyDiagnostics::default());
            }
            traversal_residency::refresh_after_traversal_runtime(
                &engine.lua,
                &mut engine.audio_manager,
                &mut game_instance,
            );
        }

        engine
    }

    /// Rebuilds the active camera from the current player position after a save is loaded.
    pub fn rebuild_camera_from_loaded_state(&mut self) {
        let mut ctx_ref = self.ctx.borrow_mut();
        let game_ref = self.game_instance.borrow();
        let ecs = &game_ref.game.ecs;
        let world = game_ref.game.current_world();
        let player_pos = ecs
            .get_player_transform()
            .map(|transform| transform.position)
            .unwrap_or_default();

        if let Some(current_room) = world.current_room() {
            self.camera_manager = CameraManager::new(
                &mut *ctx_ref,
                ecs,
                current_room.id,
                player_pos,
                world.grid_size,
            );
        }
    }

    pub fn fixed_update<C: BishopContext>(&mut self, ctx: &mut C, dt: f32) {
        let mut game_instance = self.game_instance.borrow_mut();
        game_instance.store_previous_positions(&mut self.camera_manager);

        {
            let game_ctx = game_instance.game.ctx_mut();
            let Some(world) = game_ctx.world.as_deref() else {
                return;
            };
            update_physics(game_ctx.sprite_manager, game_ctx.ecs, world, dt);
        }

        // Resolve room transitions before updating the camera
        if RoomTransitionManager::handle_transitions(&self.lua, &mut game_instance) {
            traversal_residency::refresh_after_traversal_runtime(
                &self.lua,
                &mut self.audio_manager,
                &mut game_instance,
            );
        }

        // Fire proximity WorldExits before camera update.
        WorldExitManager::handle_proximity_exits(&game_instance);

        let game_ctx = game_instance.game.ctx_mut();
        if let Some(world) = game_ctx.world.as_deref() {
            if let Some(current_room) = world.current_room() {
                self.camera_manager.update_active(
                    ctx,
                    game_ctx.ecs,
                    current_room,
                    world.grid_size,
                );
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        {
            // Keep borrow_mut in this scope
            let mut game_instance = self.game_instance.borrow_mut();

            let game_ctx = game_instance.game.ctx_mut();
            let asset_registry = game_ctx.asset_registry;
            let sprite_manager = game_ctx.sprite_manager;
            let ecs = game_ctx.ecs;

            if let Some(world) = game_ctx.world.as_deref() {
                update_speech_timers(ecs, world, dt);
                if let Some(current_room) = world.current_room() {
                    let loader = self.ctx.borrow();
                    update_animation_sytem(
                        &*loader,
                        ecs,
                        asset_registry,
                        sprite_manager,
                        dt,
                        current_room.id,
                    );
                }
            }

            // Activate scripts in this scope TODO: make this part of run_scripts when scope is finalized?
            let ctx = game_instance.game.ctx_mut();
            if let Err(e) =
                ScriptSystem::activate_entity_scripts(&self.lua, ctx.ecs, ctx.script_manager)
            {
                omni_error!("Error activating scripts: {}", e);
            }
        }

        // Sync menu state for Lua scripts
        set_menu_active(self.menu_manager.has_active_menu());

        // Run scripts outside borrow_mut scope
        if let Err(e) = ScriptSystem::run_scripts(dt, self) {
            omni_error!("Error running scripts: {}", e);
        }
    }

    pub fn render(&mut self, ctx: &PlatformContext, alpha: f32) {
        if !self.menu_manager.is_hiding_game() {
            let mut ctx_borrow = ctx.borrow_mut();
            let platform_ctx = &mut *ctx_borrow;
            let render_cam = build_render_camera(
                &self.camera_manager,
                alpha,
                platform_ctx.screen_width(),
                platform_ctx.screen_height(),
            );
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

    fn process_menu_input(&mut self, ctx: &PlatformContext) -> bool {
        let pending_action = {
            let mut ctx_ref = ctx.borrow_mut();
            self.menu_manager.handle_input(&mut *ctx_ref);
            self.menu_manager.drain_pending_session_action()
        };

        let Some(action) = pending_action else {
            return false;
        };

        match resolve_requested_session_action(action, self.quit_to_title_enabled) {
            RequestedSessionAction::QuitToTitle => {
                self.save_runtime.pending_quit_to_title.set(true);
            }
            RequestedSessionAction::CloseApp => {
                let mut ctx_ref = ctx.borrow_mut();
                ctx_ref.set_close_requested(true);
                ctx_ref.set_exit_confirmed(true);
            }
        }

        true
    }

    fn apply_pending_world_transition(&mut self) {
        let Some(request) = take_pending_world_transition() else {
            return;
        };
        let mut game_instance = self.game_instance.borrow_mut();
        if WorldTransitionManager::execute(&self.lua, &mut game_instance, &request) {
            traversal_residency::refresh_after_traversal_runtime(
                &self.lua,
                &mut self.audio_manager,
                &mut game_instance,
            );
        }
    }
}

fn resolve_requested_session_action(
    action: MenuSessionAction,
    quit_to_title_enabled: bool,
) -> RequestedSessionAction {
    match action {
        MenuSessionAction::QuitToMainMenu if quit_to_title_enabled => {
            RequestedSessionAction::QuitToTitle
        }
        MenuSessionAction::QuitToMainMenu | MenuSessionAction::QuitGame => {
            RequestedSessionAction::CloseApp
        }
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
