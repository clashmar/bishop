use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_animation;
use mlua::UserDataMethods;

pub struct IsClipFinishedMethod;

impl LuaMethod<EntityHandle> for IsClipFinishedMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_animation::IS_CLIP_FINISHED, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                if let Some(current_id) = &animation.current {
                    if let Some(state) = animation.states.get(current_id) {
                        return Ok(state.finished);
                    }
                }
            }
            Ok(false)
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Checks if the current non-looping clip has finished.");
        out.line("---@return boolean");
        out.line(&format!(
            "function Entity:{}() end",
            lua_animation::IS_CLIP_FINISHED,
        ));
        out.line("");
    }
}
