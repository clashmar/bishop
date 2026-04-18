use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct GetFlipXMethod;

impl LuaMethod<EntityHandle> for GetFlipXMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET_FLIP_X, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                Ok(animation.flip_x)
            } else {
                Ok(false)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the horizontal flip state.");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", GET_FLIP_X));
        out.line("");
    }
}
