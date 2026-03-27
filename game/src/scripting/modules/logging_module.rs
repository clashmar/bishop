// game/src/scripting/modules/logging_module.rs
use engine_core::scripting::lua_constants::*;
use engine_core::scripting::modules::lua_module::*;
use engine_core::*;
use mlua::Function;
use mlua::Lua;
use mlua::Table;
use mlua::Value;
use mlua::Variadic;

/// Log‑level strings that are exposed to Lua.
pub const LOG_INFO: &str = "info";
pub const LOG_WARN: &str = "warn";
pub const LOG_ERROR: &str = "error";
pub const LOG_DEBUG: &str = "debug";

/// Lua module that exposes the four log levels.
#[derive(Default)]
pub struct LoggingModule;
register_lua_module!(LoggingModule);

impl LuaModule for LoggingModule {
    fn register(&self, lua: &Lua) -> mlua::Result<()> {
        // Helper that creates a wrapper for a concrete log level
        fn level_wrapper(lua: &Lua, level_name: &'static str) -> mlua::Result<Function> {
            let name = level_name.to_string();
            lua.create_function(move |_lua, args: Variadic<Value>| {
                let msg = match args.iter().next() {
                    Some(Value::String(s)) => s.to_str()?.to_owned(),
                    _ => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "{name} expects a string"
                        )))
                    }
                };

                match name.as_str() {
                    LOG_INFO => onscreen_info!("[Lua] {}", msg),
                    LOG_WARN => onscreen_warn!("[Lua] {}", msg),
                    LOG_ERROR => onscreen_error!("[Lua] {}", msg),
                    LOG_DEBUG => onscreen_debug!("[Lua] {}", msg),
                    _ => onscreen_error!("[Lua] {}", "Log level from Lua was not recognised."),
                }

                Ok(Value::Nil)
            })
        }

        // Create a table that will become `engine.log`
        let log_tbl = lua.create_table()?;

        // Register the four level functions
        log_tbl.set(LOG_INFO, level_wrapper(lua, LOG_INFO)?)?;
        log_tbl.set(LOG_WARN, level_wrapper(lua, LOG_WARN)?)?;
        log_tbl.set(LOG_ERROR, level_wrapper(lua, LOG_ERROR)?)?;
        log_tbl.set(LOG_DEBUG, level_wrapper(lua, LOG_DEBUG)?)?;

        // Attach the table to the global `engine` namespace
        let globals = lua.globals();
        let engine_mod: Table = globals.get(ENGINE)?;
        engine_mod.set(LOG, log_tbl)?;

        Ok(())
    }
}

register_lua_api!(LoggingModule, ENGINE_FILE);

impl LuaApi for LoggingModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        // log.info(msg: string)
        out.line("---@param msg string");
        out.line("function engine.log.info(msg) end");
        out.line("");

        // log.warn(msg: string)
        out.line("---@param msg string");
        out.line("function engine.log.warn(msg) end");
        out.line("");

        // log.error(msg: string)
        out.line("---@param msg string");
        out.line("function engine.log.error(msg) end");
        out.line("");

        // log.debug(msg: string)
        out.line("---@param msg string");
        out.line("function engine.log.debug(msg) end");
        out.line("");
    }
}
