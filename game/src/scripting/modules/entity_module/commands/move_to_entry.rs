use crate::game_global::set_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use crate::transitions::world_transitions::{DestinationSelector, EntryHandleData, TraversalRequest};
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use engine_core::worlds::WorldTransitionMode;
use mlua::{Table, UserDataMethods};

/// Lua method recording a move of this entity to a generated entry handle.
pub struct MoveToEntryMethod;

impl LuaMethod<EntityHandle> for MoveToEntryMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_entity::MOVE_TO_ENTRY, |lua, this, entry: Table| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            let entry = EntryHandleData::from_lua(lua, entry)?;
            set_pending_world_transition(TraversalRequest {
                entity: Some(this.entity),
                destination: DestinationSelector::Entry(entry),
                mode: WorldTransitionMode::Transport,
            });
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Moves this entity to a generated entry handle destination.");
        out.line("---@param entry table");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(entry) end", lua_entity::MOVE_TO_ENTRY));
        out.line("");
    }
}
