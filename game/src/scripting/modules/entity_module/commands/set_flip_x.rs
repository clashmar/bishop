use crate::game_global::push_command;
use crate::scripting::commands::entity::SetFlipXCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct SetFlipXMethod;

impl LuaMethod<EntityHandle> for SetFlipXMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_FLIP_X, |lua, this, flip_x: bool| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(SetFlipXCmd {
                entity: this.entity,
                flip_x,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets horizontal flip for the sprite.");
        out.line("---@param flip_x boolean Whether to flip horizontally");
        out.line(&format!("function Entity:{}(flip_x) end", SET_FLIP_X));
        out.line("");
    }
}
