// game/src/scripting/modules/audio_module.rs
use engine_core::audio::runtime;
use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::Table;

/// Lua module that exposes the audio system API under `engine.audio`.
#[derive(Default)]
pub struct AudioModule;
register_lua_module!(AudioModule);

impl LuaModule for AudioModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(ENGINE)?;
        let audio_tbl = lua.create_table()?;
        let play_music_fn = lua.create_function(|_, (id, opts): (String, Option<Table>)| {
            let looping = opts
                .as_ref()
                .and_then(|t| t.get::<bool>("looping").ok())
                .unwrap_or(true);
            let fade_out = opts
                .as_ref()
                .and_then(|t| t.get::<f32>("fade_out").ok())
                .unwrap_or(0.0);

            push_audio_command(AudioCommand::PlayMusic(PlayMusicRequest {
                id,
                looping,
                fade_out,
            }));
            Ok(())
        })?;
        audio_tbl.set(AUDIO_PLAY_MUSIC, play_music_fn)?;

        let is_playing_fn = lua.create_function(|_, ()| Ok(runtime::is_music_playing()))?;
        audio_tbl.set(AUDIO_IS_PLAYING, is_playing_fn)?;

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

        let play_random_sfx_fn = lua.create_function(|_, sounds_table: Table| {
            let sounds: Vec<String> = sounds_table
                .sequence_values::<String>()
                .filter_map(|r| r.ok())
                .collect();
            if sounds.is_empty() {
                onscreen_warn!("play_random_sfx: sounds table is empty or contains no strings");
                return Ok(());
            }
            push_audio_command(AudioCommand::PlayVariedSfx {
                sounds,
                volume: 1.0,
                pitch_variation: 0.0,
                volume_variation: 0.0,
            });
            Ok(())
        })?;
        audio_tbl.set(AUDIO_PLAY_RANDOM_SFX, play_random_sfx_fn)?;

        let play_sfx_varied_fn =
            lua.create_function(|_, (id, opts): (String, Option<Table>)| {
                let pitch_variation = opts
                    .as_ref()
                    .and_then(|t| t.get::<f32>("pitch_var").ok())
                    .unwrap_or(0.0);
                let volume_variation = opts
                    .as_ref()
                    .and_then(|t| t.get::<f32>("volume_var").ok())
                    .unwrap_or(0.0);
                push_audio_command(AudioCommand::PlayVariedSfx {
                    sounds: vec![id],
                    volume: 1.0,
                    pitch_variation,
                    volume_variation,
                });
                Ok(())
            })?;
        audio_tbl.set(AUDIO_PLAY_SFX_VARIED, play_sfx_varied_fn)?;

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
        out.line("--- Plays music by ID.");
        out.line(
            "--- `opts.looping` defaults to true and `opts.fade_out` defaults to 0.0 seconds.",
        );
        out.line("---@param id string Path relative to Resources/audio/ without extension");
        out.line("---@param opts? {looping?: boolean, fade_out?: number}");
        out.line("function engine.audio.play_music(id, opts) end");
        out.line("");
        out.line("--- Returns true while music is considered active.");
        out.line("---@return boolean");
        out.line("function engine.audio.is_playing() end");
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
        out.line("--- Picks one sound at random from the list and plays it as a one-shot with no variation.");
        out.line("---@param sounds string[] Array of sound IDs");
        out.line("function engine.audio.play_random_sfx(sounds) end");
        out.line("");
        out.line("--- Plays a single sound with optional pitch and volume variation.");
        out.line("---@param id string Sound ID");
        out.line("---@param opts? {pitch_var?: number, volume_var?: number}");
        out.line("function engine.audio.play_sfx_varied(id, opts) end");
        out.line("");
    }
}
