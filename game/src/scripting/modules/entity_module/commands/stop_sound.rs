use crate::audio::scoped_playback::{authored_stop_behavior_for_entity, classify_audio_owner};
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::handle::{ensure_live_entity, EntityHandle};
use engine_core::audio::{AudioCommand, push_audio_command};
use engine_core::scripting::{LuaApiWriter, LuaMethod};
use engine_core::scripting::lua_constants::lua_audio;
use mlua::{Table, UserDataMethods};

pub struct StopSoundMethod;

impl LuaMethod<EntityHandle> for StopSoundMethod {
    fn register<M: UserDataMethods<EntityHandle>>(&self, methods: &mut M) {
        methods.add_method(lua_audio::ENTITY_STOP_SOUND, |lua, this, opts: Option<Table>| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            ensure_live_entity(&game_instance.game.ecs, this.entity)?;

            let immediate = opts
                .as_ref()
                .and_then(|t| t.get::<bool>(lua_audio::STOP_IMMEDIATE).ok())
                .unwrap_or(false);
            let fade_out = if immediate {
                None
            } else {
                opts.as_ref()
                    .and_then(|t| t.get::<f32>(lua_audio::STOP_FADE_OUT).ok())
                    .or_else(|| {
                        authored_stop_behavior_for_entity(&game_instance.game, this.entity)
                            .fade_duration()
                    })
            };

            push_audio_command(AudioCommand::StopLoops {
                owner: classify_audio_owner(&game_instance.game, this.entity),
                fade_out,
            });
            Ok(())
        });
    }

    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Stops a looping sound started by this entity's AudioSource.");
        out.line("--- Accepts an optional table with 'immediate' (bool) or 'fade_out' (number) overrides.");
        out.line("--- When called without options, uses the authored stop behavior from the AudioSource.");
        out.line(&format!(
            "function Entity:{}(opts) end",
            lua_audio::ENTITY_STOP_SOUND
        ));
        out.line("");
    }
}
