use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct StopSoundMethod;

impl LuaMethod<EntityHandle> for StopSoundMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(ENTITY_STOP_SOUND, |lua, this, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;
            push_audio_command(AudioCommand::StopLoop(*this.entity as u64));
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Stops a looping sound started by this entity's AudioSource.");
        out.line(&format!("function Entity:{}() end", ENTITY_STOP_SOUND));
        out.line("");
    }
}
