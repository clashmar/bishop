use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{
    EntityHandle, ensure_live_entity, lua_entity_handle,
};
use engine_core::ecs::get_children;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;

pub struct ChildrenMethod;

impl LuaMethod<EntityHandle> for ChildrenMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::CHILDREN, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            let table = lua.create_table()?;
            for (index, child) in get_children(ecs, this.entity).into_iter().enumerate() {
                table.set(index + 1, lua_entity_handle(lua, child)?)?;
            }
            Ok(table)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Returns this entity's children.");
        out.line("---@return Entity[]");
        out.line(&format!("function Entity:{}() end", lua_entity::CHILDREN));
        out.line("");
    }
}
