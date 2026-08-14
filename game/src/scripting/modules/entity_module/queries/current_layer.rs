use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::ecs::CurrentRoom;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;

pub struct CurrentLayerMethod;

impl LuaMethod<EntityHandle> for CurrentLayerMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::CURRENT_LAYER, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            Ok(ecs
                .get::<CurrentRoom>(this.entity)
                .map(|room| room.layer.script_name().to_string()))
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Returns the authored room layer this entity belongs to.");
        out.line("---@return string|nil");
        out.line(&format!(
            "function Entity:{}() end",
            lua_entity::CURRENT_LAYER
        ));
        out.line("");
    }
}
