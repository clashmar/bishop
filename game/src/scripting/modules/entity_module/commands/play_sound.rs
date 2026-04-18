use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_audio;
use mlua::UserDataMethods;

pub struct PlaySoundMethod;

impl LuaMethod<EntityHandle> for PlaySoundMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(
            lua_audio::ENTITY_PLAY_SOUND,
            |lua, this, group_name: String| {
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                let ecs = &game_instance.game.ecs;
                ensure_live_entity(ecs, this.entity)?;
                let Some(source) = ecs.get::<AudioSource>(this.entity) else {
                    return Ok(());
                };

                let group_id = SoundGroupId::Custom(group_name.clone());
                let Some(group) = source.groups.get(&group_id) else {
                    log::warn!(
                        "Entity {:?} tried to play missing sound group '{}'",
                        this.entity,
                        group_name
                    );
                    return Ok(());
                };
                let volume = (group.volume * source.runtime_volume).clamp(0.0, 1.0);

                if group.looping {
                    push_audio_command(AudioCommand::PlayLoop {
                        handle: *this.entity as u64,
                        sounds: group.sounds.clone(),
                        volume,
                        pitch_variation: group.pitch_variation,
                        volume_variation: group.volume_variation,
                    });
                } else {
                    push_audio_command(AudioCommand::PlayVariedSfx {
                        sounds: group.sounds.clone(),
                        volume,
                        pitch_variation: group.pitch_variation,
                        volume_variation: group.volume_variation,
                    });
                }
                Ok(())
            },
        );
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(
            "--- Plays the named sound group configured on this entity's AudioSource component.",
        );
        out.line("--- If the group is looping, starts a loop tracked by the entity ID.");
        out.line("--- If one-shot, plays with the group's pitch and volume variation.");
        out.line("---@param group_name SoundGroupId");
        out.line(&format!(
            "function Entity:{}(group_name) end",
            lua_audio::ENTITY_PLAY_SOUND
        ));
        out.line("");
    }
}
