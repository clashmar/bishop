use crate::game_global::push_command;
use crate::scripting::commands::lua_command::SetAnimSpeedCmd;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct SetAnimSpeedMethod;

impl LuaMethod<EntityHandle> for SetAnimSpeedMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(SET_ANIM_SPEED, |lua, this, speed: f32| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_command(Box::new(SetAnimSpeedCmd {
                entity: this.entity,
                speed,
            }));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Sets the animation playback speed multiplier.");
        out.line("---@param speed number Speed multiplier (1.0 = normal)");
        out.line(&format!("function Entity:{}(speed) end", SET_ANIM_SPEED));
        out.line("");
    }
}
