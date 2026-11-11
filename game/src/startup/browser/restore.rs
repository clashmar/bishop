use super::{browser_builder, load_startup_textures};
use crate::engine::{Engine, EngineBuilder, EngineEntryMode, GameInstance, PreparedGameInstance, SaveRuntime};
use crate::save_system::{RestorePhase, RuntimeSaveDocument, SaveStoreHandle};
use crate::scripting::script_system::ScriptSystem;
use crate::startup::save_restore::{apply_restore_phase, load_restore_document};
use bishop::prelude::{PlatformContext, TextureLoader};
use engine_core::game::Game;
use engine_core::storage::{load_game_shell_from_source, ResourceSource};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct PreparedBrowserRestore {
    builder: EngineBuilder,
    prepared: PreparedGameInstance,
    document: RuntimeSaveDocument,
}

/// Rebuilds a browser session from authored resources and its latest durable save.
pub fn restore_browser_game(
    source: ResourceSource,
    ctx: PlatformContext,
    save_store: SaveStoreHandle,
) -> Result<Engine, String> {
    let game = load_game_shell_from_source(&source)
        .map_err(|error| format!("browser restore shell: {error}"))?;
    let prepared = {
        let textures = ctx.borrow();
        prepare_browser_game_restore(game, source, &*textures, save_store)?
    };
    assemble_restored_browser_game(ctx, prepared)
}

pub(super) fn prepare_browser_game_restore(
    mut game: Game,
    source: ResourceSource,
    textures: &impl TextureLoader,
    save_store: SaveStoreHandle,
) -> Result<PreparedBrowserRestore, String> {
    let builder = browser_builder(&source, EngineEntryMode::Playing, save_store)?;
    game.runtime_source = Some(source.clone());
    let save_runtime = SaveRuntime::new(
        builder.save_providers.clone(), builder.pending_quit_to_title.clone(),
        builder.save_store.clone(),
    );
    let document = load_restore_document(&save_runtime, game.id)?;
    apply_restore_phase(&save_runtime, &document, RestorePhase::PreRuntime)?;
    let mut prepared = GameInstance::prepare_loaded_game(&builder.lua, game)?;
    load_startup_textures(&mut prepared, textures)?;
    ScriptSystem::init_from_source(&builder.lua, &prepared.game.script_manager.event_bus, &source)
        .map_err(|error| format!("restore main.lua: {error}"))?;
    Ok(PreparedBrowserRestore { builder, prepared, document })
}

fn assemble_restored_browser_game(
    ctx: PlatformContext,
    restore: PreparedBrowserRestore,
) -> Result<Engine, String> {
    let PreparedBrowserRestore { mut builder, prepared, document } = restore;
    let instance = {
        let mut context = ctx.borrow_mut();
        GameInstance::from_prepared(&mut *context, prepared, &mut builder.camera_manager)
    };
    let game_instance = Rc::new(RefCell::new(instance));
    let save_runtime = SaveRuntime::new(
        builder.save_providers.clone(), builder.pending_quit_to_title.clone(),
        builder.save_store.clone(),
    );
    let mut engine = builder.quit_to_title_enabled(false)
        .into_engine(game_instance, ctx, save_runtime, false)?;
    {
        let mut game_ref = engine.game_instance.borrow_mut();
        let game_ctx = game_ref.game.ctx_mut();
        ScriptSystem::activate_entity_scripts(
            &engine.lua, game_ctx.ecs, game_ctx.script_manager,
        ).map_err(|error| format!("activate restore entity scripts: {error}"))?;
    }
    apply_restore_phase(&engine.save_runtime, &document, RestorePhase::PostRuntime)?;
    ScriptSystem::process_commands(&mut engine);
    engine.rebuild_camera_from_loaded_state();
    Ok(engine)
}
