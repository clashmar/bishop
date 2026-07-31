use crate::game_global::set_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use crate::transitions::world_transitions::TraversalRequest;
use bishop::prelude::Vec2;
use engine_core::ecs::CurrentRoom;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use engine_core::worlds::{RoomId, RoomLayer};
use mlua::UserDataMethods;

pub struct MoveToRoomMethod;

impl LuaMethod<EntityHandle> for MoveToRoomMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::MOVE_TO_ROOM, |lua, this, room_id: usize| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;

            let world_id = game_instance.game.current_world().id;
            let layer = game_instance
                .game
                .ecs
                .get::<CurrentRoom>(this.entity)
                .map(|current_room| current_room.layer)
                .unwrap_or(RoomLayer::Front);
            set_pending_world_transition(TraversalRequest::restore_location(
                this.entity,
                world_id,
                RoomId(room_id),
                layer,
                Vec2::ZERO,
            ));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Moves this entity to the target room.");
        out.line("---@param room_id integer");
        out.line("---@return nil");
        out.line(&format!(
            "function Entity:{}(room_id) end",
            lua_entity::MOVE_TO_ROOM
        ));
        out.line("");
    }
}
