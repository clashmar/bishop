use crate::game_global::set_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use crate::transitions::world_transitions::{WorldSelector, WorldTransitionRequest};
use engine_core::worlds::WorldTransitionMode;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use mlua::UserDataMethods;

/// Lua method recording a move of this entity to another world.
pub struct MoveToWorldMethod;

impl LuaMethod<EntityHandle> for MoveToWorldMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(
            lua_entity::MOVE_TO_WORLD,
            |lua, this, (world_name, entry_name): (String, Option<String>)| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                ensure_live_entity(&game_instance.game.ecs, this.entity)?;
                set_pending_world_transition(WorldTransitionRequest {
                    entity: Some(this.entity),
                    world: WorldSelector::ByName(world_name),
                    entry_name,
                    mode: WorldTransitionMode::Transport,
                });
                Ok(())
            },
        );
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Moves this entity to another world.");
        out.line("--- Arrives at the named entry point, or the world's start when omitted.");
        out.line("---@param world_name string");
        out.line("---@param entry_name string|nil");
        out.line(&format!(
            "function Entity:{}(world_name, entry_name) end",
            lua_entity::MOVE_TO_WORLD
        ));
        out.line("");
    }
}
