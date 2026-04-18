use crate::game_global::push_command;
use crate::scripting::commands::entity::CallEntityFnCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::{UserDataMethods, Value, Variadic};

pub struct InteractMethod;

impl LuaMethod<EntityHandle> for InteractMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(INTERACT, |lua, this, args: Variadic<Value>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(CallEntityFnCmd {
                entity: this.entity,
                fn_name: INTERACT.to_string(),
                args: args.to_vec(),
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@vararg any Arguments passed to the entity's interact function");
        out.line("---@return nil");
        out.line(&format!("function Entity:{}(...) end", INTERACT));
        out.line("");
    }
}
