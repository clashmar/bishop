use crate::game_global::push_command;
use crate::scripting::commands::entity::MoveToRoomCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_entity;
use mlua::UserDataMethods;

pub struct MoveToRoomMethod;

impl LuaMethod<EntityHandle> for MoveToRoomMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::MOVE_TO_ROOM, |lua, this, room_id: usize| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(MoveToRoomCmd {
                entity: this.entity,
                room_id: RoomId(room_id),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Moves this entity to the target room.");
        out.line("---@param room_id integer");
        out.line(&format!(
            "function Entity:{}(room_id) end",
            lua_entity::MOVE_TO_ROOM
        ));
        out.line("");
    }
}
