use crate::game_global::push_command;
use crate::scripting::commands::entity::RemoveFromRoomCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_entity;
use mlua::UserDataMethods;

pub struct RemoveFromRoomMethod;

impl LuaMethod<EntityHandle> for RemoveFromRoomMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::REMOVE_FROM_ROOM, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(RemoveFromRoomCmd {
                entity: this.entity,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Removes this entity from its current room membership.");
        out.line(&format!(
            "function Entity:{}() end",
            lua_entity::REMOVE_FROM_ROOM
        ));
        out.line("");
    }
}
