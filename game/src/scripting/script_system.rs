// engine_core/src/script/script_system.rs
use crate::engine::Engine;
use crate::game_global::drain_commands;
use crate::scripting::modules::entity_module::*;
use engine_core::prelude::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::{Function, Table};
use std::fs;
use std::sync::Arc;

/// Registry key for the global update function from main.lua.
const GLOBAL_UPDATE_KEY: &str = "__global_update";

pub struct ScriptSystem;

impl ScriptSystem {
    /// Initialize the script system.
    pub fn init(lua: &Lua, event_bus: &EventBus) {
        // Registers the `engine` module that some other modules extend
        if let Err(e) = Self::register_engine_module(lua) {
            onscreen_error!("Error registering engine module: {e}")
        };

        // Store the event bus in Lua so engine.on/engine.emit work at require-time
        if let Err(e) = lua.globals().set(LUA_EVENT_BUS, event_bus.clone()) {
            onscreen_error!("Failed to store event bus: {e}");
        }

        // Sub-modules — register before main.lua so all APIs are available at require-time
        for descriptor in inventory::iter::<LuaModuleRegistry> {
            let module = (descriptor.ctor)();
            if let Err(e) = module.register(lua) {
                onscreen_error!("Lua module registration failed: {e}");
            }
        }

        // Run main.lua after all modules are registered
        if let Err(e) = Self::load_main(lua) {
            onscreen_error!("Main failed: {e}");
        }

        // Store the global update function if main.lua set engine.update
        if let Ok(engine_tbl) = lua.globals().get::<Table>(ENGINE) {
            if let Ok(update_fn) = engine_tbl.get::<Function>(UPDATE) {
                if let Err(e) = lua.set_named_registry_value(GLOBAL_UPDATE_KEY, update_fn) {
                    onscreen_error!("Failed to store global update: {e}");
                }
            }
        }
    }

    /// Loads and executes main.lua if present.
    fn load_main(lua: &Lua) -> LuaResult<()> {
        let main_path = scripts_folder().join(MAIN_FILE);
        let src =
            fs::read_to_string(main_path).map_err(|e| mlua::Error::ExternalError(Arc::new(e)))?;
        lua.load(&src).exec()
    }

    /// Called during init.
    fn register_engine_module(lua: &Lua) -> LuaResult<()> {
        let engine_mod = lua.create_table()?;
        lua.globals().set(ENGINE, engine_mod.clone())?;
        lua.register_module(ENGINE, &engine_mod)?;
        Ok(())
    }

    /// Runs all lua scripts in the game.
    pub fn run_scripts(dt: f32, engine: &mut Engine) -> LuaResult<()> {
        // Collect all pending inits and their functions in a single borrow
        let inits_to_run: Vec<(Function, Table)> = {
            let mut game_instance = engine.game_instance.borrow_mut();
            let script_manager = &mut game_instance.game.script_manager;

            let pending = std::mem::take(&mut script_manager.pending_inits);

            pending
                .into_iter()
                .filter_map(|(entity, script_id)| {
                    let instance = script_manager.instances.get(&(entity, script_id))?;
                    let init_fn = instance.get::<Function>(INIT).ok()?;
                    Some((init_fn.clone(), instance.clone()))
                })
                .collect()
        };

        for (init_fn, instance) in inits_to_run {
            init_fn.call::<()>(&instance)?;
            Self::process_commands(engine);
        }

        // Collect all scripts to run in a single borrow
        let scripts_to_run: Vec<(Function, Table)> = {
            let game_instance = engine.game_instance.borrow();
            let ctx = game_instance.game.ctx();
            let script_manager = &game_instance.game.script_manager;
            let script_store = ctx.ecs.get_store::<Script>();

            script_store
                .data
                .iter()
                .filter_map(|(entity, script)| {
                    if script.script_id == ScriptId(0) {
                        return None;
                    }

                    let update_fn = script_manager.update_fns.get(&script.script_id)?;
                    let instance = script_manager.instances.get(&(*entity, script.script_id))?;

                    Some((update_fn.clone(), instance.clone()))
                })
                .collect()
        };

        // Execute without holding any borrows
        for (update_fn, instance) in scripts_to_run {
            update_fn.call::<()>((instance, dt))?;
            Self::process_commands(engine);
        }

        // Call the global update function from main.lua if one was defined
        if let Ok(global_update) = engine
            .lua
            .named_registry_value::<Function>(GLOBAL_UPDATE_KEY)
        {
            global_update.call::<()>(dt)?;
            Self::process_commands(engine);
        }

        Ok(())
    }

    /// Process all Lua commands to the `Engine`.
    pub fn process_commands(engine: &mut Engine) {
        // Drain the command queue and apply each command
        for mut cmd in drain_commands() {
            cmd.execute(engine);
        }
    }

    /// Initializes all needed scripts in the game.
    /// Only creates entity handles and queues init for newly created instances.
    pub fn load_scripts(
        lua: &Lua,
        ecs: &mut Ecs,
        script_manager: &mut ScriptManager,
    ) -> LuaResult<()> {
        let script_store = ecs.get_store_mut::<Script>();

        for (entity, script) in script_store.data.iter_mut() {
            if script.script_id == ScriptId(0) {
                continue;
            }

            let (instance, created) =
                script_manager.get_or_create_instance(lua, *entity, script.script_id)?;

            // Only setup entity handle and queue init for newly created instances
            if created {
                let handle = lua_entity_handle(lua, *entity)?;
                instance.set(ENTITY, handle)?;

                let has_init = instance.get::<Function>(INIT).is_ok();

                // Use sync_to_lua_with_instance to avoid redundant lookup
                script.sync_to_lua_with_instance(lua, instance)?;

                if has_init {
                    script_manager
                        .pending_inits
                        .push((*entity, script.script_id));
                }
            }
        }

        Ok(())
    }
}
