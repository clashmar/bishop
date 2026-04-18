use crate::game_global::push_command;
use crate::scripting::commands::entity::MoveEntityByCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::MOVE_BY;
use engine_core::scripting::parse_named_vec2;
use mlua::{Table, UserDataMethods};

pub struct MoveByMethod;

impl LuaMethod<EntityHandle> for MoveByMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(MOVE_BY, |lua, this, delta: Table| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            let delta = parse_named_vec2(&delta, &format!("Entity:{MOVE_BY} delta"))?;
            push_command(Box::new(MoveEntityByCmd {
                entity: this.entity,
                delta,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Instantly offsets the entity by a world-space delta.");
        out.line("---@param delta vec2");
        out.line(&format!("function Entity:{}(delta) end", MOVE_BY));
        out.line("");
    }
}
