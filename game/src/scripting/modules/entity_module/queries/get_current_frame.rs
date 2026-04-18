use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_animation;
use mlua::{UserDataMethods, Value};

pub struct GetCurrentFrameMethod;

impl LuaMethod<EntityHandle> for GetCurrentFrameMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_animation::GET_CURRENT_FRAME, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            if let Some(frame) = ecs.get::<CurrentFrame>(this.entity) {
                let table = lua.create_table()?;
                table.set("col", frame.col)?;
                table.set("row", frame.row)?;
                Ok(Value::Table(table))
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the current animation frame indices.");
        out.line("---@return {col: integer, row: integer}|nil");
        out.line(&format!(
            "function Entity:{}() end",
            lua_animation::GET_CURRENT_FRAME,
        ));
        out.line("");
    }
}
