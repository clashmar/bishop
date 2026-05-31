use super::game_instance::GameInstance;
use super::save_runtime::SaveRuntime;
use super::{Engine, EngineEntryMode};
use crate::save_system::SaveProviderRegistry;
use crate::scripting::lua_ctx::{register_runtime_lua_contexts, register_save_lua_context};
use bishop::prelude::*;
use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Shared resources for constructing an Engine, with early save-context registration.
pub struct EngineBuilder {
    pub lua: Lua,
    pub camera_manager: CameraManager,
    pub save_providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    pub pending_quit_to_title: Rc<Cell<bool>>,
    quit_to_title_enabled: bool,
    entry_mode: EngineEntryMode,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineBuilder {
    /// Creates a new builder with a fresh Lua VM and empty provider registry.
    pub fn new() -> Self {
        Self {
            lua: Lua::new(),
            camera_manager: CameraManager::default(),
            save_providers: Rc::new(RefCell::new(SaveProviderRegistry::new())),
            pending_quit_to_title: Rc::new(Cell::new(false)),
            quit_to_title_enabled: true,
            entry_mode: EngineEntryMode::Playing,
        }
    }

    /// Sets the entry mode for the loaded session.
    pub fn entry_mode(mut self, entry_mode: EngineEntryMode) -> Self {
        self.entry_mode = entry_mode;
        self
    }

    pub(crate) fn quit_to_title_enabled(mut self, enabled: bool) -> Self {
        self.quit_to_title_enabled = enabled;
        self
    }

    /// Registers only LuaSaveCtx so bootstrap scripts can register save providers.
    pub fn register_save_context(&self) -> LuaResult<()> {
        register_save_lua_context(&self.lua, self.save_providers.clone(), self.pending_quit_to_title.clone())
    }

    /// Final assembly with pre-built runtime pieces.
    ///
    /// Registers runtime Lua contexts and creates the final Engine.
    pub fn into_engine(
        self,
        game_instance: Rc<RefCell<GameInstance>>,
        ctx: PlatformContext,
        save_runtime: SaveRuntime,
        is_playtest: bool,
    ) -> Engine {
        let grid_size = game_instance.borrow().game.current_world().grid_size;
        if let Err(e) = register_runtime_lua_contexts(
            &self.lua,
            game_instance.clone(),
            ctx.clone(),
        ) {
            onscreen_error!("Could not register lua contexts: {}", e);
        }
        Engine::new(
            game_instance,
            ctx,
            self.lua,
            save_runtime,
            self.camera_manager,
            grid_size,
            is_playtest,
            self.quit_to_title_enabled,
            self.entry_mode,
        )
    }

    /// Convenience helper for the raw startup path.
    pub fn assemble(
        self,
        game_instance: GameInstance,
        ctx: PlatformContext,
        is_playtest: bool,
    ) -> Engine {
        let save_runtime = SaveRuntime::new(self.save_providers.clone(), self.pending_quit_to_title.clone());
        let game_instance = Rc::new(RefCell::new(game_instance));
        self.into_engine(game_instance, ctx, save_runtime, is_playtest)
    }
}
