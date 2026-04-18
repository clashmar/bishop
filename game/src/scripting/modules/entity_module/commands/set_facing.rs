use crate::game_global::push_command;
use crate::scripting::commands::lua_command::{parse_direction, SetFacingCmd};
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct SetFacingMethod;

impl LuaMethod<EntityHandle> for SetFacingMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_FACING, |lua, this, direction: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            let direction = parse_direction(&direction).map_err(mlua::Error::RuntimeError)?;
            push_command(Box::new(SetFacingCmd {
                entity: this.entity,
                direction,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the facing direction (for auto-flip with mirrored clips).");
        out.line("---@param direction Direction|string");
        out.line(&format!("function Entity:{}(direction) end", SET_FACING));
        out.line("");
    }
}
