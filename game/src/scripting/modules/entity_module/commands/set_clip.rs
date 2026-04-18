use crate::game_global::push_command;
use crate::scripting::commands::entity::SetClipCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_animation;
use mlua::UserDataMethods;

pub struct SetClipMethod;

impl LuaMethod<EntityHandle> for SetClipMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_animation::SET_CLIP, |lua, this, clip_name: String| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(SetClipCmd {
                entity: this.entity,
                clip_name,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the active animation clip.");
        out.line("---@param clip_name string The name of the clip (e.g. \"Walk\", \"Idle\")");
        out.line(&format!(
            "function Entity:{}(clip_name) end",
            lua_animation::SET_CLIP
        ));
        out.line("");
    }
}
