use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{
    EntityHandle, ensure_live_entity, lua_entity_handle,
};
use engine_core::ecs::get_parent;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::{UserDataMethods, Value};

pub struct ParentMethod;

impl LuaMethod<EntityHandle> for ParentMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::PARENT, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            match get_parent(ecs, this.entity) {
                Some(parent) => lua_entity_handle(lua, parent),
                None => Ok(Value::Nil),
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Returns this entity's parent.");
        out.line("---@return Entity|nil");
        out.line(&format!("function Entity:{}() end", lua_entity::PARENT));
        out.line("");
    }
}
