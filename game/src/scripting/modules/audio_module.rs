// game/src/scripting/modules/audio_module.rs
use engine_core::audio::{push_audio_command, AudioCommand};
use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::modules::lua_module::*;
use engine_core::scripting::lua_constants::*;
use mlua::prelude::LuaResult;
use mlua::Table;
use mlua::Lua;

/// Lua module that exposes the audio system API under `engine.audio`.
#[derive(Default)]
pub struct AudioModule;
register_lua_module!(AudioModule);

impl LuaModule for AudioModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(ENGINE)?;
        let audio_tbl = lua.create_table()?;

        let play_music_fn = lua.create_function(|_, id: String| {
            push_audio_command(AudioCommand::PlayMusic(id));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_PLAY_MUSIC, play_music_fn)?;

        let stop_music_fn = lua.create_function(|_, ()| {
            push_audio_command(AudioCommand::StopMusic);
            Ok(())
        })?;
        audio_tbl.set(AUDIO_STOP_MUSIC, stop_music_fn)?;

        let fade_music_fn = lua.create_function(|_, duration: f32| {
            push_audio_command(AudioCommand::FadeMusic(duration));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_FADE_MUSIC, fade_music_fn)?;

        let play_sfx_fn = lua.create_function(|_, id: String| {
            push_audio_command(AudioCommand::PlaySfx(id));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_PLAY_SFX, play_sfx_fn)?;

        let preload_fn = lua.create_function(|_, id: String| {
            push_audio_command(AudioCommand::Preload(id));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_PRELOAD, preload_fn)?;

        let set_master_volume_fn = lua.create_function(|_, v: f32| {
            push_audio_command(AudioCommand::SetMasterVolume(v));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_SET_MASTER_VOLUME, set_master_volume_fn)?;

        let set_music_volume_fn = lua.create_function(|_, v: f32| {
            push_audio_command(AudioCommand::SetMusicVolume(v));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_SET_MUSIC_VOLUME, set_music_volume_fn)?;

        let set_sfx_volume_fn = lua.create_function(|_, v: f32| {
            push_audio_command(AudioCommand::SetSfxVolume(v));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_SET_SFX_VOLUME, set_sfx_volume_fn)?;

        let unload_fn = lua.create_function(|_, id: String| {
            push_audio_command(AudioCommand::Unload(id));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_UNLOAD, unload_fn)?;

        engine_tbl.set(LUA_AUDIO, audio_tbl)?;
        Ok(())
    }
}

register_lua_api!(AudioModule, AUDIO_FILE);

impl LuaApi for AudioModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Audio system module");
        out.line("---@class AudioApi");
        out.line("engine.audio = {}");
        out.line("");
        out.line("--- Plays music by ID, looping until stopped. Stops any current track.");
        out.line("---@param id string Path relative to Resources/audio/ without extension");
        out.line("function engine.audio.play_music(id) end");
        out.line("");
        out.line("--- Stops music immediately.");
        out.line("function engine.audio.stop_music() end");
        out.line("");
        out.line("--- Fades music out over duration seconds, then stops.");
        out.line("---@param duration number Fade duration in seconds");
        out.line("function engine.audio.fade_music(duration) end");
        out.line("");
        out.line("--- Plays a sound effect fire-and-forget.");
        out.line("---@param id string Path relative to Resources/audio/ without extension");
        out.line("function engine.audio.play_sfx(id) end");
        out.line("");
        out.line("--- Pre-loads a sound into the cache to prevent stutter on first play.");
        out.line("---@param id string Path relative to Resources/audio/ without extension");
        out.line("function engine.audio.preload(id) end");
        out.line("");
        out.line("--- Sets master volume (0.0–1.0).");
        out.line("---@param volume number");
        out.line("function engine.audio.set_master_volume(volume) end");
        out.line("");
        out.line("--- Sets music group volume (0.0–1.0).");
        out.line("---@param volume number");
        out.line("function engine.audio.set_music_volume(volume) end");
        out.line("");
        out.line("--- Sets SFX group volume (0.0–1.0).");
        out.line("---@param volume number");
        out.line("function engine.audio.set_sfx_volume(volume) end");
        out.line("");
        out.line("--- Unpins a preloaded sound and evicts it from the cache if no components reference it.");
        out.line("---@param id string Path relative to Resources/audio/ without extension");
        out.line("function engine.audio.unload(id) end");
        out.line("");
    }
}
