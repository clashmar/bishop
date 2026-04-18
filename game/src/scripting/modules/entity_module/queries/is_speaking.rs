use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_text;
use mlua::UserDataMethods;

pub struct IsSpeakingMethod;

impl LuaMethod<EntityHandle> for IsSpeakingMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_text::IS_SPEAKING, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            Ok(ecs.has::<SpeechBubble>(this.entity))
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Checks if the entity currently has a speech bubble.");
        out.line("---@return boolean");
        out.line(&format!("function Entity:{}() end", lua_text::IS_SPEAKING));
        out.line("");
    }
}
