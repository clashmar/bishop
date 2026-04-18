use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::{UserDataMethods, Value};

pub struct GetClipMethod;

impl LuaMethod<EntityHandle> for GetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(GET_CLIP, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;

            if let Some(animation) = ecs.get::<Animation>(this.entity) {
                if let Some(clip_id) = &animation.current {
                    Ok(Value::String(lua.create_string(clip_id.ui_label())?))
                } else {
                    Ok(Value::Nil)
                }
            } else {
                Ok(Value::Nil)
            }
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Gets the current animation clip name.");
        out.line("---@return string|nil");
        out.line(&format!("function Entity:{}() end", GET_CLIP));
        out.line("");
    }
}
