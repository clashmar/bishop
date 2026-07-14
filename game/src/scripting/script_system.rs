use crate::engine::Engine;
use crate::game_global::drain_commands;
use crate::scripting::modules::entity_module::lua_entity_handle;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::logging::{omni_error};
use engine_core::scripting::{EventBus, LuaModuleRegistry, ScriptManager, register_runtime_modules};
use engine_core::storage::*;
use engine_core::scripting::lua_constants::{lua_engine, lua_entity, lua_files, lua_globals};
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::{Function, Table, Value};
use std::fs;
use std::sync::Arc;

/// Registry key for the global update function from main.lua.
const GLOBAL_UPDATE_KEY: &str = "__global_update";
type ScriptCallback = (Entity, ScriptId, Option<String>, Function, Table);

pub struct ScriptSystem;

impl ScriptSystem {
    /// Initialize the script system.
    pub fn init(lua: &Lua, event_bus: &EventBus) {
        if let Err(e) = register_runtime_modules(lua, event_bus) {
            omni_error!("Lua module registration failed: {e}");
        }

        if let Err(e) = Self::register_game_modules(lua) {
            omni_error!("Lua game module registration failed: {e}");
        }

        ScriptManager::load_to_package(lua);

        // Run main.lua after all modules are registered
        if let Err(e) = Self::load_main(lua) {
            omni_error!("Main failed: {e}");
        }

        // Store the global update function if main.lua set engine.update
        if let Ok(engine_tbl) = lua.globals().get::<Table>(lua_engine::ENGINE) {
            if let Ok(update_fn) = engine_tbl.get::<Function>(lua_entity::UPDATE) {
                if let Err(e) = lua.set_named_registry_value(GLOBAL_UPDATE_KEY, update_fn) {
                    omni_error!("Failed to store global update: {e}");
                }
            }
        }
    }

    /// Registers game-specific runtime modules after shared bootstrap has run.
    fn register_game_modules(lua: &Lua) -> LuaResult<()> {
        for descriptor in inventory::iter::<LuaModuleRegistry> {
            let module = (descriptor.ctor)();
            module.register(lua)?;
        }

        Ok(())
    }

    /// Loads and executes main.lua if present.
    fn load_main(lua: &Lua) -> LuaResult<()> {
        ScriptManager::load_globals_prelude(lua)?;

        let main_path = scripts_folder().join(lua_files::MAIN);
        let src =
            fs::read_to_string(main_path).map_err(|e| mlua::Error::ExternalError(Arc::new(e)))?;
        lua.load(&src).exec()
    }

    /// Runs all lua scripts in the game.
    pub fn run_scripts(dt: f32, engine: &mut Engine) -> LuaResult<()> {
        // Collect all pending inits and their functions in a single borrow
        let inits_to_run: Vec<ScriptCallback> = {
            let mut game_instance = engine.game_instance.borrow_mut();
            let script_manager = &mut game_instance.game.script_manager;

            let pending = std::mem::take(&mut script_manager.pending_inits);

            pending
                .into_iter()
                .filter_map(|(entity, script_id)| {
                    let instance = script_manager.instances.get(&(entity, script_id))?;
                    let init_fn = instance.get::<Function>(lua_entity::INIT).ok()?;
                    let script_path = script_manager
                        .path_for_id(script_id)
                        .map(|path| path.display().to_string());
                    Some((
                        entity,
                        script_id,
                        script_path,
                        init_fn.clone(),
                        instance.clone(),
                    ))
                })
                .collect()
        };

        Self::run_entity_init_callbacks(inits_to_run, || Self::process_commands(engine));

        // Collect all scripts to run in a single borrow
        let scripts_to_run: Vec<ScriptCallback> = {
            let game_instance = engine.game_instance.borrow();
            let ctx = game_instance.game.ctx();
            let script_manager = &game_instance.game.script_manager;
            let script_store = ctx.ecs.get_store::<Script>();

            script_store
                .data
                .iter()
                .filter_map(|(entity, script)| {
                    if !update_eligible(&game_instance.game, *entity, script) {
                        return None;
                    }

                    let update_fn = script_manager.update_fns.get(&script.script_id)?;
                    let instance = script_manager.instances.get(&(*entity, script.script_id))?;
                    let script_path = script_manager
                        .path_for_id(script.script_id)
                        .map(|path| path.display().to_string());

                    Some((
                        *entity,
                        script.script_id,
                        script_path,
                        update_fn.clone(),
                        instance.clone(),
                    ))
                })
                .collect()
        };

        // Execute without holding any borrows
        for (entity, script_id, script_path, update_fn, instance) in scripts_to_run {
            {
                let game_instance = engine.game_instance.borrow();
                if !script_update_is_still_valid(&game_instance.game.ecs, entity, script_id) {
                    continue;
                }
            }

            Self::run_entity_update_callback(
                entity,
                script_id,
                script_path.as_deref(),
                &update_fn,
                &instance,
                dt,
                || Self::process_commands(engine),
            );
        }

        // Call the global update function from main.lua if one was defined
        if let Ok(global_update) = engine.lua.named_registry_value::<Function>(GLOBAL_UPDATE_KEY) {
            Self::run_global_update_callback(&global_update, dt, || Self::process_commands(engine));
        }

        Ok(())
    }

    /// Process all Lua commands and queued menu-open callbacks to completion.
    pub fn process_commands(engine: &mut Engine) {
        loop {
            let mut did_work = false;

            for mut cmd in drain_commands() {
                did_work = true;
                cmd.execute(engine);
            }

            if let Some(callback_path) = engine.menu_manager.take_pending_on_open() {
                did_work = true;
                if let Err(e) = invoke_menu_callback(&engine.lua, &callback_path) {
                    omni_error!("menu on_open callback failed: {e}");
                }
            }

            if !did_work {
                break;
            }
        }
    }

    /// Activates scripts for every eligible entity in the game.
    /// Only active entities create instances and queue init work.
    pub fn activate_entity_scripts(
        lua: &Lua,
        ecs: &mut Ecs,
        script_manager: &mut ScriptManager,
    ) -> LuaResult<()> {
        let entities = ecs
            .get_store::<Script>()
            .data
            .keys()
            .copied()
            .collect::<Vec<_>>();

        for entity in entities {
            Self::activate_entity_script(lua, ecs, script_manager, entity)?;
        }

        Ok(())
    }

    /// Tears down payload-scoped script instances, queued init work, and entity listeners.
    pub fn deactivate_payload_scripts(
        ecs: &Ecs,
        script_manager: &mut ScriptManager,
        entities: &[Entity],
    ) {
        script_manager.discard_pending_inits_for_entities(entities);
        for &entity in entities {
            if ecs.get::<Script>(entity).is_some() {
                script_manager.unload_all_for_entity(entity);
            }
        }
    }

    /// Activates payload-scoped script instances for the supplied resident entities.
    pub fn activate_payload_scripts(
        lua: &Lua,
        ecs: &mut Ecs,
        script_manager: &mut ScriptManager,
        entities: &[Entity],
    ) -> LuaResult<()> {
        for &entity in entities {
            Self::activate_entity_script(lua, ecs, script_manager, entity)?;
        }
        Ok(())
    }

    /// Prepares immediate init calls for a freshly spawned prefab subtree.
    pub fn prepare_spawned_script_inits(
        lua: &Lua,
        ecs: &mut Ecs,
        script_manager: &mut ScriptManager,
        root_entity: Entity,
        root_args: Option<Value>,
    ) -> LuaResult<Vec<(Function, Table, Option<Value>)>> {
        let mut entities = Vec::new();
        collect_prefab_subtree(ecs, root_entity, &mut entities);
        let mut root_has_script = false;
        let mut root_has_init = false;
        let mut inits = Vec::new();

        for entity in entities {
            let Some(script) = ecs.get::<Script>(entity).cloned() else {
                continue;
            };
            if script.script_id == ScriptId(0) {
                continue;
            }

            if entity == root_entity {
                root_has_script = true;
            }

            let (instance, created) =
                script_manager.get_or_create_instance(lua, entity, script.script_id)?;
            if !created {
                continue;
            }

            let handle = lua_entity_handle(lua, entity)?;
            instance.set(lua_globals::ENTITY_HANDLE, handle)?;
            script.sync_to_lua_with_instance(lua, instance)?;

            if let Ok(init_fn) = instance.get::<Function>(lua_entity::INIT) {
                let args = if entity == root_entity {
                    root_has_init = true;
                    root_args.clone()
                } else {
                    None
                };
                inits.push((init_fn.clone(), instance.clone(), args));
            }
        }

        if root_args.is_some() && !root_has_script {
            return Err(mlua::Error::RuntimeError(
                "engine.prefab.spawn init requires a Script on the prefab root".into(),
            ));
        }

        if root_args.is_some() && !root_has_init {
            return Err(mlua::Error::RuntimeError(
                "engine.prefab.spawn init requires a root script init(self, init)".into(),
            ));
        }

        Ok(inits)
    }

    fn run_entity_init_callbacks(
        callbacks: Vec<ScriptCallback>,
        mut after_each: impl FnMut(),
    ) {
        for (entity, script_id, script_path, init_fn, instance) in callbacks {
            if let Err(error) = init_fn.call::<()>(&instance) {
                log_entity_script_error(
                    "init",
                    entity,
                    script_id,
                    script_path.as_deref(),
                    &error,
                );
            }
            after_each();
        }
    }

    fn run_entity_update_callback(
        entity: Entity,
        script_id: ScriptId,
        script_path: Option<&str>,
        update_fn: &Function,
        instance: &Table,
        dt: f32,
        mut after_each: impl FnMut(),
    ) {
        if let Err(error) = update_fn.call::<()>((instance.clone(), dt)) {
            log_entity_script_error("update", entity, script_id, script_path, &error);
        }
        after_each();
    }

    fn run_global_update_callback(
        global_update: &Function,
        dt: f32,
        mut after_each: impl FnMut(),
    ) {
        if let Err(error) = global_update.call::<()>(dt) {
            omni_error!("global engine.update failed: {error}");
        }
        after_each();
    }

    fn activate_entity_script(
        lua: &Lua,
        ecs: &mut Ecs,
        script_manager: &mut ScriptManager,
        entity: Entity,
    ) -> LuaResult<()> {
        let Some(script) = ecs.get::<Script>(entity).cloned() else {
            return Ok(());
        };
        if script.script_id == ScriptId(0) {
            return Ok(());
        }
        if !ecs.get::<Active>(entity).is_some_and(Active::is_enabled) {
            return Ok(());
        }

        let (instance, created) =
            script_manager.get_or_create_instance(lua, entity, script.script_id)?;
        if !created {
            return Ok(());
        }

        let handle = lua_entity_handle(lua, entity)?;
        instance.set(lua_globals::ENTITY_HANDLE, handle)?;

        let has_init = instance.get::<Function>(lua_entity::INIT).is_ok();
        script.sync_to_lua_with_instance(lua, instance)?;

        if has_init && !script_manager.pending_inits.contains(&(entity, script.script_id)) {
            script_manager.pending_inits.push((entity, script.script_id));
        }

        Ok(())
    }
}

fn invoke_menu_callback(lua: &Lua, callback_path: &str) -> LuaResult<()> {
    let (module_name, path) = callback_path.split_once('.').ok_or_else(|| {
        mlua::Error::RuntimeError(format!(
            "menu on_open callback '{callback_path}' must be in 'module.function' form"
        ))
    })?;

    let require: Function = lua.globals().get("require")?;
    let mut value = require.call::<Value>(module_name)?;

    for segment in path.split('.') {
        let Value::Table(table) = value else {
            return Err(mlua::Error::RuntimeError(format!(
                "menu on_open callback '{callback_path}' path '{segment}' is not a table/function path"
            )));
        };
        value = table.get::<Value>(segment)?;
    }

    let Value::Function(callback) = value else {
        return Err(mlua::Error::RuntimeError(format!(
            "menu on_open callback '{callback_path}' did not resolve to a function"
        )));
    };

    callback.call(())
}

fn collect_prefab_subtree(ecs: &Ecs, root_entity: Entity, entities: &mut Vec<Entity>) {
    entities.push(root_entity);
    for child in get_children(ecs, root_entity) {
        collect_prefab_subtree(ecs, child, entities);
    }
}

/// An entity's script updates only when it has a real script, is active, and is in the active world.
fn update_eligible(game: &Game, entity: Entity, script: &Script) -> bool {
    script.script_id != ScriptId(0)
        && game.entity_in_active_world(entity)
        && game.ecs.get::<Active>(entity).is_some_and(Active::is_enabled)
}

fn script_update_is_still_valid(ecs: &Ecs, entity: Entity, script_id: ScriptId) -> bool {
    ecs.get::<Script>(entity)
        .is_some_and(|script| script.script_id == script_id)
}

fn log_entity_script_error(
    phase: &str,
    entity: Entity,
    script_id: ScriptId,
    script_path: Option<&str>,
    error: &mlua::Error,
) {
    match script_path {
        Some(path) => omni_error!(
            "script {phase} failed for entity {:?}, script {:?} ({}): {}",
            entity,
            script_id,
            path,
            error
        ),
        None => omni_error!(
            "script {phase} failed for entity {:?}, script {:?}: {}",
            entity,
            script_id,
            error
        ),
    }
}

#[cfg(test)]
mod tests;
