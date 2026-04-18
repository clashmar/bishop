use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct GetMethod;

impl LuaMethod<EntityHandle> for GetMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET, |lua, this, comp_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let entity = this.entity;
            ensure_live_entity(ecs, entity)?;

            let reg = public_lua_component(&comp_name).map_err(mlua::Error::RuntimeError)?;
            if (reg.has)(ecs, entity) {
                let boxed = (reg.clone)(ecs, entity);
                (reg.to_lua)(lua, &*boxed)
            } else {
                Err(mlua::Error::RuntimeError(format!(
                    "Entity {:?} has no {} component",
                    entity, comp_name
                )))
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("-- Component getters");
        for reg in public_lua_components() {
            out.line(&format!(
                "---@overload fun(self: Entity, component: \"{}\"): {}",
                reg.type_name, reg.type_name
            ));
        }
        out.line("---@param component ComponentId");
        out.line("---@return table|nil");
        out.line(&format!("function Entity:{}(component) end", GET));
        out.line("");
    }
}
