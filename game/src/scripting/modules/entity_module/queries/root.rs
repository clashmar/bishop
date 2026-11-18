use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{
    EntityHandle, ensure_live_entity, lua_entity_handle,
};
use engine_core::ecs::{Ecs, Entity, get_parent};
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;
use std::collections::HashSet;

pub struct RootMethod;

impl LuaMethod<EntityHandle> for RootMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::ROOT, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            let root = root_entity(ecs, this.entity)?;
            lua_entity_handle(lua, root)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Returns this entity's hierarchy root.");
        out.line("---@return Entity");
        out.line(&format!("function Entity:{}() end", lua_entity::ROOT));
        out.line("");
    }
}

fn root_entity(ecs: &Ecs, entity: Entity) -> mlua::Result<Entity> {
    let mut visited = HashSet::new();
    let mut current = entity;

    while let Some(parent) = get_parent(ecs, current) {
        if !visited.insert(current) {
            return Err(mlua::Error::RuntimeError(format!(
                "Entity hierarchy cycle detected at {}",
                *current
            )));
        }
        current = parent;
    }

    Ok(current)
}
