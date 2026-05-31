// engine_core/src/script/script_system.rs
use crate::engine::Engine;
use crate::game_global::drain_commands;
use crate::scripting::modules::entity_module::lua_entity_handle;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{lua_dirs, lua_engine, lua_entity, lua_files, lua_globals};
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::{Function, Table, Value};
use std::fs;
use std::sync::Arc;

/// Registry key for the global update function from main.lua.
const GLOBAL_UPDATE_KEY: &str = "__global_update";

pub struct ScriptSystem;

impl ScriptSystem {
    /// Initialize the script system.
    pub fn init(lua: &Lua, event_bus: &EventBus) {
        if let Err(e) = register_runtime_modules(lua, event_bus) {
            onscreen_error!("Lua module registration failed: {e}");
        }

        if let Err(e) = Self::register_game_modules(lua) {
            onscreen_error!("Lua game module registration failed: {e}");
        }

        ScriptManager::load_to_package(lua);

        // Run main.lua after all modules are registered
        if let Err(e) = Self::load_main(lua) {
            onscreen_error!("Main failed: {e}");
        }

        // Store the global update function if main.lua set engine.update
        if let Ok(engine_tbl) = lua.globals().get::<Table>(lua_engine::ENGINE) {
            if let Ok(update_fn) = engine_tbl.get::<Function>(lua_entity::UPDATE) {
                if let Err(e) = lua.set_named_registry_value(GLOBAL_UPDATE_KEY, update_fn) {
                    onscreen_error!("Failed to store global update: {e}");
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

    fn load_globals(lua: &Lua) -> LuaResult<()> {
        let globals_path = scripts_folder()
            .join(lua_dirs::ENGINE)
            .join(lua_files::GLOBALS);
        let src = match fs::read_to_string(globals_path) {
            Ok(src) => src,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(mlua::Error::ExternalError(Arc::new(error))),
        };
        lua.load(&src).exec()
    }

    /// Loads and executes main.lua if present.
    fn load_main(lua: &Lua) -> LuaResult<()> {
        Self::load_globals(lua)?;

        let main_path = scripts_folder().join(lua_files::MAIN);
        let src =
            fs::read_to_string(main_path).map_err(|e| mlua::Error::ExternalError(Arc::new(e)))?;
        lua.load(&src).exec()
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
                    let init_fn = instance.get::<Function>(lua_entity::INIT).ok()?;
                    Some((init_fn.clone(), instance.clone()))
                })
                .collect()
        };

        for (init_fn, instance) in inits_to_run {
            init_fn.call::<()>(&instance)?;
            Self::process_commands(engine);
        }

        // Collect all scripts to run in a single borrow
        let scripts_to_run: Vec<(Entity, ScriptId, Function, Table)> = {
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

                    Some((
                        *entity,
                        script.script_id,
                        update_fn.clone(),
                        instance.clone(),
                    ))
                })
                .collect()
        };

        // Execute without holding any borrows
        for (entity, script_id, update_fn, instance) in scripts_to_run {
            {
                let game_instance = engine.game_instance.borrow();
                if !script_update_is_still_valid(&game_instance.game.ecs, entity, script_id) {
                    continue;
                }
            }

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
                    onscreen_error!("menu on_open callback failed: {e}");
                }
            }

            if !did_work {
                break;
            }
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
                instance.set(lua_globals::ENTITY_HANDLE, handle)?;

                let has_init = instance.get::<Function>(lua_entity::INIT).is_ok();

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

fn script_update_is_still_valid(ecs: &Ecs, entity: Entity, script_id: ScriptId) -> bool {
    ecs.get::<Script>(entity)
        .is_some_and(|script| script.script_id == script_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::constants::paths;
    use engine_core::engine_global::set_game_name;
    use engine_core::scripting::lua_constants::{lua_dirs, lua_fields, lua_files};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::fs;

    fn install_callback_module(lua: &Lua) {
        lua.load(
            r#"
            callback_hits = 0
            package.preload["save_manager"] = function()
                return {
                    on_title_menu_open = function()
                        callback_hits = callback_hits + 1
                    end,
                    nested = {
                        on_open = function()
                            callback_hits = callback_hits + 1
                        end,
                    },
                }
            end
            "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn script_update_eligibility_rejects_entities_without_the_same_script_component() {
        let mut ecs = Ecs::default();
        let entity = ecs.create_entity().finish();

        assert!(!script_update_is_still_valid(&ecs, entity, ScriptId(7)));

        ecs.add_component_to_entity(
            entity,
            Script {
                script_id: ScriptId(3),
                ..Default::default()
            },
        );

        assert!(!script_update_is_still_valid(&ecs, entity, ScriptId(7)));
        assert!(script_update_is_still_valid(&ecs, entity, ScriptId(3)));
    }

    #[test]
    fn invoke_menu_callback_calls_exported_function() {
        let lua = Lua::new();
        install_callback_module(&lua);

        invoke_menu_callback(&lua, "save_manager.on_title_menu_open").unwrap();

        assert_eq!(lua.globals().get::<i64>("callback_hits").unwrap(), 1);
    }

    #[test]
    fn invoke_menu_callback_supports_nested_table_paths() {
        let lua = Lua::new();
        install_callback_module(&lua);

        invoke_menu_callback(&lua, "save_manager.nested.on_open").unwrap();

        assert_eq!(lua.globals().get::<i64>("callback_hits").unwrap(), 1);
    }

    #[test]
    fn init_executes_globals_prelude_before_main() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("script_system_globals_prelude");
        set_game_name(test_game.name());

        let scripts_dir = game_folder(test_game.name())
            .join(paths::RESOURCES_FOLDER)
            .join(paths::SCRIPTS_FOLDER);
        let engine_dir = scripts_dir.join(lua_dirs::ENGINE);
        fs::create_dir_all(&engine_dir).unwrap();
        fs::write(
            engine_dir.join(lua_files::GLOBALS),
            "bootstrap_order = (bootstrap_order or \"\") .. \"g\"\nInput = { Space = \"space\" }\n",
        )
        .unwrap();
        fs::write(
            scripts_dir.join(lua_files::MAIN),
            "bootstrap_order = bootstrap_order .. \"m\"\nsaw_input = Input.Space\n",
        )
        .unwrap();

        set_game_name(test_game.name());
        let lua = Lua::new();
        let event_bus = EventBus::default();
        ScriptSystem::init(&lua, &event_bus);

        assert_eq!(lua.globals().get::<String>("bootstrap_order").unwrap(), "gm");
        assert_eq!(lua.globals().get::<String>("saw_input").unwrap(), "space");
    }

    #[test]
    fn prepare_spawned_script_inits_rejects_root_args_without_root_init() {
        let lua = Lua::new();
        let mut ecs = Ecs::default();
        let root = ecs.create_entity().finish();
        let mut script_manager = ScriptManager::default();
        let def = lua.create_table().unwrap();
        let public = lua.create_table().unwrap();
        let init_args = lua.create_table().unwrap();

        public.set("speed", 120).unwrap();
        def.set(lua_fields::PUBLIC, public).unwrap();
        init_args.set("direction", "left").unwrap();
        script_manager.table_defs.insert(ScriptId(1), def);
        ecs.add_component_to_entity(
            root,
            Script {
                script_id: ScriptId(1),
                ..Default::default()
            },
        );

        let error = ScriptSystem::prepare_spawned_script_inits(
            &lua,
            &mut ecs,
            &mut script_manager,
            root,
            Some(Value::Table(init_args)),
        )
        .unwrap_err();

        assert!(error.to_string().contains("root script init"));
    }
}
