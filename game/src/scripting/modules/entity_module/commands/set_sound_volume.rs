use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use mlua::UserDataMethods;

pub struct SetSoundVolumeMethod;

impl LuaMethod<EntityHandle> for SetSoundVolumeMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(ENTITY_SET_SOUND_VOLUME, |lua, this, v: f32| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let mut game_instance = ctx.game_instance.borrow_mut();
            let ecs = &mut game_instance.game.ecs;
            ensure_live_entity(ecs, this.entity)?;
            if let Some(source) = ecs.get_mut::<AudioSource>(this.entity) {
                source.runtime_volume = v.clamp(0.0, 1.0);
            }
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(
            "--- Sets a runtime gain multiplier on this entity's AudioSource groups (0.0–1.0).",
        );
        out.line("--- Takes effect on the next play_sound() call.");
        out.line("---@param v number Volume in range 0.0–1.0");
        out.line(&format!(
            "function Entity:{}(v) end",
            ENTITY_SET_SOUND_VOLUME
        ));
        out.line("");
    }
}
