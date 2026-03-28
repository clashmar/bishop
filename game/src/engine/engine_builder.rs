// game/src/engine/engine_builder.rs
use super::game_instance::GameInstance;
use super::Engine;
use crate::scripting::lua_ctx::register_lua_contexts;
use bishop::prelude::*;
use engine_core::prelude::*;
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;

/// Holds shared resources needed to construct an [`Engine`], allowing the caller
/// to choose which [`GameInstance`] constructor to use.
pub struct EngineBuilder {
    pub lua: Lua,
    pub camera_manager: CameraManager,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineBuilder {
    /// Creates a new builder with a fresh Lua VM and default camera manager.
    pub fn new() -> Self {
        Self {
            lua: Lua::new(),
            camera_manager: CameraManager::default(),
        }
    }

    /// Wraps `game_instance`, extracts `grid_size`, registers Lua contexts,
    /// and constructs the [`Engine`].
    pub fn assemble(
        self,
        game_instance: GameInstance,
        ctx: PlatformContext,
        is_playtest: bool,
    ) -> Engine {
        let grid_size = game_instance.game.current_world().grid_size;
        let game_instance = Rc::new(RefCell::new(game_instance));
        if let Err(e) = register_lua_contexts(&self.lua, game_instance.clone(), ctx.clone()) {
            onscreen_error!("Could not register lua contexts: {}", e);
        }
        Engine::new(
            game_instance,
            ctx,
            self.lua,
            self.camera_manager,
            grid_size,
            is_playtest,
        )
    }
}
