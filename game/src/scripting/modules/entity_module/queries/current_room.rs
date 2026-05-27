use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_entity;
use mlua::UserDataMethods;

pub struct CurrentRoomMethod;

impl LuaMethod<EntityHandle> for CurrentRoomMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::CURRENT_ROOM, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            Ok(ecs
                .get::<CurrentRoom>(this.entity)
                .map(|room| mlua::Value::Integer(room.0 .0 as i64)))
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Returns the room this entity belongs to.");
        out.line("---@return integer|nil");
        out.line(&format!(
            "function Entity:{}() end",
            lua_entity::CURRENT_ROOM
        ));
        out.line("");
    }
}
