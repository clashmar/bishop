// game/src/scripting/modules/engine_module.rs
use crate::scripting::lua_ctx::LuaGameCtx;
use engine_core::prelude::*;
use engine_core::register_lua_api;
use engine_core::register_lua_module;
use mlua::prelude::LuaResult;
use mlua::Function;
use mlua::Lua;
use mlua::MultiValue;
use mlua::Table;
use mlua::Value;
use mlua::Variadic;

/// Lua module that exposes the engine's global modules to scripts.
#[derive(Default)]
pub struct EngineModule;
register_lua_module!(EngineModule);

impl LuaModule for EngineModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(ENGINE)?;

        // TODO: assess if this is needed
        // Create the metatable for global entity proxies
        let proxy_mt = lua.create_table()?;

        // __index metamethod - intercepts method/field access
        let index_fn = lua.create_function(|lua, (proxy_table, key): (Table, String)| {
            // Get the entity name stored in the proxy
            let entity_name: String = proxy_table.raw_get("__entity_name")?;

            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let script_manager = &game_instance.game.script_manager;

            // Find global entity by name
            let global_entity = {
                let global_store = ecs.get_store::<Global>();
                let name_store = ecs.get_store::<Name>();

                global_store
                    .data
                    .keys()
                    .find(|&&entity| {
                        name_store
                            .get(entity)
                            .map(|n| n.0 == entity_name)
                            .unwrap_or(false)
                    })
                    .copied()
            };

            let entity = global_entity.ok_or_else(|| {
                mlua::Error::RuntimeError(format!("Global entity '{}' not found", entity_name))
            })?;

            // Get the script component
            let script_id = ecs
                .get_store::<Script>()
                .get(entity)
                .map(|s| s.script_id)
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!(
                        "Global entity '{}' has no script",
                        entity_name
                    ))
                })?;

            // Get the script instance
            let instance = script_manager
                .instances
                .get(&(entity, script_id))
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!(
                        "Script instance not found for global '{}'",
                        entity_name
                    ))
                })?;

            // Try to get the value from the instance
            match instance.get::<Value>(key.clone()) {
                Ok(Value::Function(_func)) => {
                    // Return a wrapper function that calls with self
                    let entity_name_clone = entity_name.clone();
                    let wrapper = lua.create_function(move |lua, args: Variadic<Value>| {
                        let ctx = LuaGameCtx::borrow_ctx(lua)?;
                        let game_instance = ctx.game_instance.borrow();
                        let ecs = &game_instance.game.ecs;
                        let script_manager = &game_instance.game.script_manager;

                        // Re-lookup entity and instance
                        let entity = {
                            let global_store = ecs.get_store::<Global>();
                            let name_store = ecs.get_store::<Name>();

                            global_store
                                .data
                                .keys()
                                .find(|&&e| {
                                    name_store
                                        .get(e)
                                        .map(|n| n.0 == entity_name_clone)
                                        .unwrap_or(false)
                                })
                                .copied()
                                .ok_or_else(|| {
                                    mlua::Error::RuntimeError(format!(
                                        "Global entity '{}' not found",
                                        entity_name_clone
                                    ))
                                })?
                        };

                        let script_id = ecs
                            .get_store::<Script>()
                            .get(entity)
                            .map(|s| s.script_id)
                            .ok_or_else(|| {
                                mlua::Error::RuntimeError(format!(
                                    "Global entity '{}' has no script",
                                    entity_name_clone
                                ))
                            })?;

                        let instance = script_manager
                            .instances
                            .get(&(entity, script_id))
                            .ok_or_else(|| {
                                mlua::Error::RuntimeError(format!(
                                    "Script instance not found for global '{}'",
                                    entity_name_clone
                                ))
                            })?;

                        let func = instance.get::<Function>(key.clone())?;

                        // Build call args with instance as first argument (self)
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(Value::Table(instance.clone()));
                        call_args.extend(args.into_iter());

                        func.call::<MultiValue>(MultiValue::from_vec(call_args))
                    })?;

                    Ok(Value::Function(wrapper))
                }
                Ok(value) => Ok(value),
                Err(_) => {
                    // Try public table
                    if let Ok(public_tbl) = instance.get::<Table>(PUBLIC) {
                        public_tbl.get::<Value>(key)
                    } else {
                        Ok(Value::Nil)
                    }
                }
            }
        })?;
        proxy_mt.set("__index", index_fn)?;

        // Store the metatable in registry for reuse
        lua.set_named_registry_value("__global_proxy_mt", proxy_mt.clone())?;

        // engine.global(name) - creates a proxy for a global entity
        let global_fn = lua.create_function(|lua, name: String| {
            let proxy = lua.create_table()?;
            proxy.raw_set("__entity_name", name)?;

            // Get the metatable from registry
            let mt: Table = lua.named_registry_value("__global_proxy_mt")?;
            let _ = proxy.set_metatable(Some(mt));

            Ok(proxy)
        })?;
        engine_tbl.set(GLOBAL, global_fn)?;

        // engine.player() - returns the player entity's script instance table
        let player_fn = lua.create_function(|lua, ()| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let script_manager = &game_instance.game.script_manager;

            // Find player entity via Player component
            let player_entity = ecs.get_store::<Player>().data.keys().next().copied();

            let entity = match player_entity {
                Some(e) => e,
                None => return Ok(Value::Nil),
            };

            // Get script component
            let script_id = match ecs.get_store::<Script>().get(entity) {
                Some(s) => s.script_id,
                None => return Ok(Value::Nil),
            };

            // Get script instance
            let instance = match script_manager.instances.get(&(entity, script_id)) {
                Some(i) => i.clone(),
                None => return Ok(Value::Nil),
            };

            Ok(Value::Table(instance))
        })?;
        engine_tbl.set("player", player_fn)?;

        // global.call(name, method_name, ...)
        let call_fn = lua.create_function(|lua, args: Variadic<Value>| {
            // Extract global entity name and method name
            let mut iter = args.into_iter();

            let name = match iter.next() {
                Some(Value::String(s)) => s.to_str()?.to_owned(),
                _ => {
                    return Err(mlua::Error::RuntimeError(
                        "First argument must be the global entity name (string)".into(),
                    ))
                }
            };

            let method = match iter.next() {
                Some(Value::String(s)) => s.to_str()?.to_owned(),
                _ => {
                    return Err(mlua::Error::RuntimeError(
                        "Second argument must be the method name (string)".into(),
                    ))
                }
            };

            // Remaining args go to the method
            let method_args: Vec<Value> = iter.collect();

            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let script_manager = &game_instance.game.script_manager;

            // Find global entity by name
            let global_entity = {
                let global_store = ecs.get_store::<Global>();
                let name_store = ecs.get_store::<Name>();

                global_store
                    .data
                    .keys()
                    .find(|&&entity| name_store.get(entity).map(|n| n.0 == name).unwrap_or(false))
                    .copied()
            };

            let entity = global_entity.ok_or_else(|| {
                mlua::Error::RuntimeError(format!("Global entity '{}' not found", name))
            })?;

            // Get the script component
            let script_id = ecs
                .get_store::<Script>()
                .get(entity)
                .map(|s| s.script_id)
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!("Global entity '{}' has no script", name))
                })?;

            // Get the script instance
            let instance = script_manager
                .instances
                .get(&(entity, script_id))
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!(
                        "Script instance not found for global '{}'",
                        name
                    ))
                })?;

            // Get the method function
            let func = instance.get::<Function>(method.clone()).map_err(|_| {
                mlua::Error::RuntimeError(format!(
                    "Method '{}' not found on global '{}'",
                    method, name
                ))
            })?;

            // Build call args with instance as first argument (self)
            let handle = Value::Table(instance.clone());
            let mut call_args = Vec::with_capacity(method_args.len() + 1);
            call_args.push(handle);
            call_args.extend(method_args);

            // Execute the call synchronously and return the result
            func.call::<MultiValue>(MultiValue::from_vec(call_args))
        })?;
        engine_tbl.set(ENGINE_CALL, call_fn)?;

        // engine.on(event, handler)
        let on_fn = lua.create_function(|lua, (event, handler): (String, Function)| {
            let ud: mlua::AnyUserData = lua.globals().get(LUA_EVENT_BUS)?;
            let bus = ud.borrow::<EventBus>()?;
            bus.on(event, handler);
            Ok(())
        })?;
        engine_tbl.set(ENGINE_ON, on_fn)?;

        // engine.emit(event, …)
        let emit_fn = lua.create_function(|lua, (event, args): (String, Variadic<Value>)| {
            let ud: mlua::AnyUserData = lua.globals().get(LUA_EVENT_BUS)?;
            let bus = ud.borrow::<EventBus>()?;
            bus.emit(event, args);
            Ok(())
        })?;
        engine_tbl.set(ENGINE_EMIT, emit_fn)?;
        Ok(())
    }
}

register_lua_api!(EngineModule, ENGINE_FILE);

impl LuaApi for EngineModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        // engine.player
        out.line("--- Get the player entity's script instance table");
        out.line("--- @return table|nil The player's script instance, or nil if not found");
        out.line("function engine.player() end");
        out.line("");

        // engine.call
        out.line("--- Call a method on a global entity script");
        out.line("--- @param name string The name of the global entity");
        out.line("--- @param method string The method name to call");
        out.line("--- @param ... any Additional arguments to pass to the method");
        out.line("--- @return any Returns whatever the method returns");
        out.line("function engine.call(name, method, ...) end");
        out.line("");

        // engine.on
        out.line("--- Register an event handler");
        out.line("--- @param event string The name of the event to listen for");
        out.line("--- @param handler function The Lua function that will be called");
        out.line("--- @return nil");
        out.line("function engine.on(event, handler) end");
        out.line("");

        // engine.emit
        out.line("--- Emit an event to all registered handlers");
        out.line("--- @param event string The name of the event to emit");
        out.line("--- @param ... any Arguments that will be passed to each handler");
        out.line("--- @return nil");
        out.line("function engine.emit(event, ...) end");
        out.line("");
    }
}
