use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{lua_entity_handle, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_entity;
use mlua::{UserDataMethods, Value};

pub struct FindBestInteractableMethod;

impl LuaMethod<EntityHandle> for FindBestInteractableMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::FIND_BEST_INTERACTABLE, |lua, _this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            if let Some(entity) = find_best_interactable(ecs) {
                lua_entity_handle(lua, entity)
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@return Entity|nil");
        out.line(&format!(
            "function Entity:{}() end",
            lua_entity::FIND_BEST_INTERACTABLE,
        ));
        out.line("");
    }
}
