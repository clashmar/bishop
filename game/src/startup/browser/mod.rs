mod asset;
mod restore;
#[cfg(test)]
mod tests;

pub use self::restore::restore_browser_game;
use self::asset::browser_entry_mode;
use crate::engine::{Engine, EngineBuilder, EngineEntryMode, GameInstance, PreparedGameInstance};
use crate::save_system::SaveStoreHandle;
use crate::scripting::script_system::ScriptSystem;
use bishop::prelude::{PlatformContext, TextureLoader};
use engine_core::assets::AssetKey;
use engine_core::game::Game;
use engine_core::scripting::install_prefetched;
use engine_core::storage::ResourceSource;
use std::cell::RefCell;
use std::rc::Rc;

/// Game data and Luau state prepared before a browser GPU context is assembled.
pub struct PreparedBrowserGame {
    pub(crate) builder: EngineBuilder,
    pub(crate) prepared: PreparedGameInstance,
}

/// Validates authored resources and prepares the existing game instance for browser play.
pub fn prepare_browser_game(
    game: Game,
    source: ResourceSource,
    textures: &impl TextureLoader,
    save_store: SaveStoreHandle,
) -> Result<PreparedBrowserGame, String> {
    let entry_mode = browser_entry_mode(&source)?;
    prepare_browser_game_with_entry_mode(game, source, textures, entry_mode, save_store)
}

/// Prepares a direct-play developer checkpoint without the authored title flow.
pub fn prepare_browser_game_direct(
    game: Game,
    source: ResourceSource,
    textures: &impl TextureLoader,
    save_store: SaveStoreHandle,
) -> Result<PreparedBrowserGame, String> {
    prepare_browser_game_with_entry_mode(game, source, textures, EngineEntryMode::Playing, save_store)
}

fn prepare_browser_game_with_entry_mode(
    mut game: Game,
    source: ResourceSource,
    textures: &impl TextureLoader,
    entry_mode: EngineEntryMode,
    save_store: SaveStoreHandle,
) -> Result<PreparedBrowserGame, String> {
    game.runtime_source = Some(source.clone());
    let builder = browser_builder(&source, entry_mode, save_store)?;
    let mut prepared = GameInstance::prepare_loaded_game(&builder.lua, game)?;
    load_startup_textures(&mut prepared, textures)?;

    ScriptSystem::init_from_source(&builder.lua, &prepared.game.script_manager.event_bus, &source)
        .map_err(|error| format!("startup main.lua: {error}"))?;
    let game_ctx = prepared.game.ctx_mut();
    ScriptSystem::activate_entity_scripts(&builder.lua, game_ctx.ecs, game_ctx.script_manager)
        .map_err(|error| format!("activate startup entity scripts: {error}"))?;
    Ok(PreparedBrowserGame { builder, prepared })
}

/// Assembles the prepared game with the real Bishop browser context and Engine.
pub fn assemble_browser_game(
    ctx: PlatformContext,
    prepared: PreparedBrowserGame,
) -> Result<Engine, String> {
    let PreparedBrowserGame { mut builder, prepared } = prepared;
    let instance = {
        let mut context = ctx.borrow_mut();
        GameInstance::from_prepared(&mut *context, prepared, &mut builder.camera_manager)
    };
    let game_instance = Rc::new(RefCell::new(instance));
    let save_runtime = crate::engine::SaveRuntime::new(
        builder.save_providers.clone(), builder.pending_quit_to_title.clone(),
        builder.save_store.clone(),
    );
    let mut engine = builder.quit_to_title_enabled(false)
        .into_engine(game_instance, ctx, save_runtime, false)?;
    engine.rebuild_camera_from_loaded_state();
    Ok(engine)
}

fn browser_builder(
    source: &ResourceSource,
    entry_mode: EngineEntryMode,
    save_store: SaveStoreHandle,
) -> Result<EngineBuilder, String> {
    if !matches!(source, ResourceSource::Prefetched(_)) {
        return Err("browser game requires prefetched authored resources".into());
    }
    let builder = EngineBuilder::new().save_store(save_store).entry_mode(entry_mode);
    builder.register_save_context()
        .map_err(|error| format!("register save Lua context: {error}"))?;
    install_prefetched(&builder.lua, source.prefetched_lua_sources()
        .map_err(|error| format!("prefetched Lua sources: {error}"))?)
        .map_err(|error| format!("install Luau modules: {error}"))?;
    Ok(builder)
}

fn load_startup_textures(
    prepared: &mut PreparedGameInstance,
    textures: &impl TextureLoader,
) -> Result<(), String> {
    let mut sprite_ids: Vec<_> = prepared.game.asset_registry.records().keys()
        .filter_map(|asset| match asset {
            AssetKey::Sprite(id) => Some(*id),
            _ => None,
        })
        .collect();
    sprite_ids.sort_by_key(|id| id.0);
    for id in sprite_ids {
        prepared.game.sprite_manager.ensure_loaded(textures, id)
            .map_err(|error| format!("startup texture {id:?}: {error}"))?;
    }
    Ok(())
}
