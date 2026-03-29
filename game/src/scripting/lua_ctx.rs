// game/src/scripting/lua_ctx.rs
use crate::engine::game_instance::GameInstance;
use bishop::prelude::*;
use engine_core::scripting::lua_constants::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::UserData;
use mlua::UserDataRef;
use std::cell::RefCell;
use std::rc::Rc;

/// The Lua constant for the bishop context.
pub const LUA_BISHOP_CTX: &str = "BISHOP_CTX";

/// The Lua‑exposed game context that gives scripts access to the current `GameState`.
#[derive(Clone)]
pub struct LuaGameCtx {
    pub game_instance: Rc<RefCell<GameInstance>>,
}

impl UserData for LuaGameCtx {}

impl LuaGameCtx {
    /// Registers this `LuaGameCtx` instance in the Lua global table.
    pub fn set_lua_ctx(self, lua: &Lua) -> LuaResult<()> {
        lua.globals().set(LUA_GAME_CTX, self)?;
        Ok(())
    }

    /// Retrieves a borrowed reference to the stored `LuaGameCtx`.
    pub fn borrow_ctx(lua: &Lua) -> LuaResult<UserDataRef<LuaGameCtx>> {
        let user_data: mlua::AnyUserData = lua.globals().get(LUA_GAME_CTX)?;
        user_data.borrow::<LuaGameCtx>()
    }
}

/// The Lua‑exposed bishop context that gives scripts access to the current `BishopContext`.
#[derive(Clone)]
pub struct LuaBishopCtx {
    pub ctx: PlatformContext,
}

impl UserData for LuaBishopCtx {}

impl LuaBishopCtx {
    /// Registers this `LuaBishopCtx` instance in the Lua global table.
    pub fn set_lua_ctx(self, lua: &Lua) -> LuaResult<()> {
        lua.globals().set(LUA_BISHOP_CTX, self)
    }

    /// Retrieves a borrowed reference to the stored `LuaBishopCtx`.
    pub fn borrow_ctx(lua: &Lua) -> LuaResult<UserDataRef<LuaBishopCtx>> {
        let user_data: mlua::AnyUserData = lua.globals().get(LUA_BISHOP_CTX)?;
        user_data.borrow::<LuaBishopCtx>()
    }
}

/// Registers both game and bishop contexts in the Lua global table.
pub fn register_lua_contexts(
    lua: &Lua,
    game_instance: Rc<RefCell<GameInstance>>,
    ctx: PlatformContext,
) -> LuaResult<()> {
    LuaGameCtx { game_instance }.set_lua_ctx(lua)?;
    LuaBishopCtx { ctx }.set_lua_ctx(lua)?;
    Ok(())
}
