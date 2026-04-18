use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::EntityHandle;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::DESPAWN;
use mlua::UserDataMethods;

pub struct DespawnMethod;

impl LuaMethod<EntityHandle> for DespawnMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(DESPAWN, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let mut game_instance = ctx.game_instance.borrow_mut();
            let mut game_ctx = game_instance.game.ctx_mut();
            Ecs::remove_entity(&mut game_ctx, this.entity);
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return nil");
        out.line(&format!("function Entity:{}() end", DESPAWN));
        out.line("");
    }
}
