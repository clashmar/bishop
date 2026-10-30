#[cfg(test)]
mod tests;

use crate::engine::{Engine, EngineBuilder, EngineEntryMode, GameInstance, PreparedGameInstance};
use crate::scripting::script_system::ScriptSystem;
use crate::transitions::world_transitions::{TraversalRequest, WorldSelector, WorldTransitionManager};
use bishop::prelude::{PlatformContext, TextureLoader};
use engine_core::assets::AssetKey;
use engine_core::ecs::{CurrentRoom, Transform};
use engine_core::game::Game;
use engine_core::scripting::install_prefetched;
use engine_core::storage::ResourceSource;
use std::cell::RefCell;
use std::rc::Rc;
use mlua::Lua;

/// Game data and Luau state prepared before a browser GPU context is assembled.
pub struct PreparedBrowserGame {
    pub(crate) builder: EngineBuilder,
    pub(crate) prepared: PreparedGameInstance,
}

/// Validates authored resources and prepares the existing game instance for browser play.
pub fn prepare_browser_game(
    mut game: Game,
    source: ResourceSource,
    textures: &impl TextureLoader,
) -> Result<PreparedBrowserGame, String> {
    if !matches!(&source, ResourceSource::Prefetched(_)) {
        return Err("browser game requires prefetched authored resources".into());
    }
    game.runtime_source = Some(source.clone());
    let builder = EngineBuilder::new();
    builder.register_save_context()
        .map_err(|error| format!("register save Lua context: {error}"))?;
    install_prefetched(&builder.lua, source.prefetched_lua_sources()
        .map_err(|error| format!("prefetched Lua sources: {error}"))?)
        .map_err(|error| format!("install Luau modules: {error}"))?;

    let mut prepared = GameInstance::prepare_loaded_game(&builder.lua, game)?;
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
    let mut instance = {
        let mut context = ctx.borrow_mut();
        GameInstance::from_prepared(&mut *context, prepared, &mut builder.camera_manager)
    };
    place_browser_player_at_start(&builder.lua, &mut instance)?;
    let game_instance = Rc::new(RefCell::new(instance));
    let save_runtime = crate::engine::SaveRuntime::new(
        builder.save_providers.clone(), builder.pending_quit_to_title.clone(),
    );
    let mut engine = builder.entry_mode(EngineEntryMode::Playing).quit_to_title_enabled(false)
        .into_engine(game_instance, ctx, save_runtime, false)?;
    engine.rebuild_camera_from_loaded_state();
    Ok(engine)
}

/// Uses the authored start entry to move a serialized, unroomed player into the room.
pub(super) fn place_browser_player_at_start(lua: &Lua, instance: &mut GameInstance) -> Result<(), String> {
    let game = &instance.game;
    let world = game.current_world();
    let room_id = world.current_room_id.ok_or("browser start has no active room")?;
    let world_id = world.id;
    let player = game.ecs.get_player_entity().ok_or("browser start has no player entity")?;
    if game.ecs.get::<Transform>(player).is_none() {
        return Err(format!("browser player {player:?} has no transform"));
    }
    if game.ecs.get::<CurrentRoom>(player).is_some_and(|room| room.room_id == room_id) {
        return Ok(());
    }
    let request = TraversalRequest::transport(player, WorldSelector::ById(world_id), None);
    if !WorldTransitionManager::execute(lua, instance, &request) {
        return Err(format!("could not place browser player {player:?} at start entry in world {world_id:?}"));
    }
    Ok(())
}
