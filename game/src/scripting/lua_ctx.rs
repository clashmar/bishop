use crate::engine::game_instance::GameInstance;
use crate::save_system::SaveProviderRegistry;
use bishop::prelude::*;
use engine_core::scripting::lua_constants::lua_globals;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::UserData;
use mlua::UserDataRef;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Lua global key for the bishop context.
pub const LUA_BISHOP_CTX: &str = "BISHOP_CTX";

/// Lua-exposed game context for script access to `GameState`.
#[derive(Clone)]
pub struct LuaGameCtx {
    pub game_instance: Rc<RefCell<GameInstance>>,
}

impl UserData for LuaGameCtx {}

impl LuaGameCtx {
    /// Registers this `LuaGameCtx` in the Lua global table.
    pub fn set_lua_ctx(self, lua: &Lua) -> LuaResult<()> {
        lua.globals().set(lua_globals::LUA_GAME_CTX, self)?;
        Ok(())
    }

    /// Borrows the stored `LuaGameCtx` from Lua globals.
    pub fn borrow_ctx(lua: &Lua) -> LuaResult<UserDataRef<LuaGameCtx>> {
        let user_data: mlua::AnyUserData = lua.globals().get(lua_globals::LUA_GAME_CTX)?;
        user_data.borrow::<LuaGameCtx>()
    }
}

/// Lua-exposed bishop context for script access to `BishopContext`.
#[derive(Clone)]
pub struct LuaBishopCtx {
    pub ctx: PlatformContext,
}

impl UserData for LuaBishopCtx {}

impl LuaBishopCtx {
    /// Registers this `LuaBishopCtx` in the Lua global table.
    pub fn set_lua_ctx(self, lua: &Lua) -> LuaResult<()> {
        lua.globals().set(LUA_BISHOP_CTX, self)
    }

    /// Borrows the stored `LuaBishopCtx` from Lua globals.
    pub fn borrow_ctx(lua: &Lua) -> LuaResult<UserDataRef<LuaBishopCtx>> {
        let user_data: mlua::AnyUserData = lua.globals().get(LUA_BISHOP_CTX)?;
        user_data.borrow::<LuaBishopCtx>()
    }
}

/// Lua global key for the save context.
pub const LUA_SAVE_CTX: &str = "LUA_SAVE_CTX";

/// Lua-exposed save context for script access to `SaveProviderRegistry`.
#[derive(Clone)]
pub struct LuaSaveCtx {
    pub save_providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    pub pending_quit_to_title: Rc<Cell<bool>>,
}

impl UserData for LuaSaveCtx {}

impl LuaSaveCtx {
    /// Registers this `LuaSaveCtx` in the Lua global table.
    pub fn set_lua_ctx(self, lua: &Lua) -> LuaResult<()> {
        lua.globals().set(LUA_SAVE_CTX, self)
    }

    /// Borrows the stored `LuaSaveCtx` from Lua globals.
    pub fn borrow_ctx(lua: &Lua) -> LuaResult<UserDataRef<LuaSaveCtx>> {
        let user_data: mlua::AnyUserData = lua.globals().get(LUA_SAVE_CTX)?;
        user_data.borrow::<LuaSaveCtx>()
    }
}

/// Registers LuaSaveCtx early so bootstrap-time scripts can register save providers.
pub fn register_save_lua_context(
    lua: &Lua,
    save_providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    pending_quit_to_title: Rc<Cell<bool>>,
) -> LuaResult<()> {
    LuaSaveCtx {
        save_providers,
        pending_quit_to_title,
    }
    .set_lua_ctx(lua)?;
    Ok(())
}

/// Registers LuaGameCtx and LuaBishopCtx after the game world is loaded.
pub fn register_runtime_lua_contexts(
    lua: &Lua,
    game_instance: Rc<RefCell<GameInstance>>,
    ctx: PlatformContext,
) -> LuaResult<()> {
    LuaGameCtx { game_instance }.set_lua_ctx(lua)?;
    LuaBishopCtx { ctx }.set_lua_ctx(lua)?;
    Ok(())
}

/// Registers all Lua contexts: save, game, and bishop.
pub fn register_lua_contexts(
    lua: &Lua,
    game_instance: Rc<RefCell<GameInstance>>,
    save_providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    ctx: PlatformContext,
) -> LuaResult<()> {
    register_save_lua_context(lua, save_providers, Rc::new(Cell::new(false)))?;
    register_runtime_lua_contexts(lua, game_instance, ctx)?;
    Ok(())
}
