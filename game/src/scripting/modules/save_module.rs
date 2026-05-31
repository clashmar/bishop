use crate::game_global::push_command;
use crate::save_system::{LuaSaveProvider, SaveLane};
use crate::scripting::commands::save_commands::{LoadLatestSaveCmd, SaveToLaneCmd};
use crate::scripting::lua_ctx::LuaSaveCtx;
use engine_core::prelude::*;
use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields, lua_files, lua_save};
use mlua::prelude::LuaResult;
use mlua::Function;
use mlua::Lua;
use mlua::Table;
use serde_json;

/// Registers a save provider from a Lua table definition.
///
/// Validates that all required fields (`id`, `version`, `capture`, `apply`) are
/// present and of the correct type before touching the save provider registry.
fn register_provider(lua: &Lua, def: Table) -> LuaResult<()> {
    let id: String = def
        .get(lua_fields::ID)
        .map_err(|_| mlua::Error::RuntimeError("save provider 'id' field is missing or not a string".into()))?;
    let version: u32 = def
        .get(lua_save::PROVIDER_VERSION)
        .map_err(|_| mlua::Error::RuntimeError("save provider 'version' field is missing or not a number".into()))?;
    let capture: Function = def
        .get(lua_save::PROVIDER_CAPTURE)
        .map_err(|_| mlua::Error::RuntimeError("save provider 'capture' function is missing or not a function".into()))?;
    let apply: Function = def
        .get(lua_save::PROVIDER_APPLY)
        .map_err(|_| mlua::Error::RuntimeError("save provider 'apply' function is missing or not a function".into()))?;

    let provider = LuaSaveProvider::new(lua, id.clone(), version, capture, apply)?;
    LuaSaveCtx::borrow_ctx(lua)?
        .save_providers
        .borrow_mut()
        .register(Box::new(provider))
        .map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
    Ok(())
}

/// Lua module exposing `engine.save` helpers to scripts.
#[derive(Default)]
pub struct SaveModule;
register_lua_module!(SaveModule);
register_lua_api!(SaveModule, lua_files::SAVE);

impl LuaModule for SaveModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
        let save_tbl = lua.create_table()?;

        save_tbl.set(
            lua_save::MANUAL,
            lua.create_function(|_, ()| {
                push_command(Box::new(SaveToLaneCmd(SaveLane::Manual)));
                Ok(())
            })?,
        )?;
        save_tbl.set(
            lua_save::AUTO,
            lua.create_function(|_, ()| {
                push_command(Box::new(SaveToLaneCmd(SaveLane::Autosave)));
                Ok(())
            })?,
        )?;
        save_tbl.set(
            lua_save::CHECKPOINT,
            lua.create_function(|_, ()| {
                push_command(Box::new(SaveToLaneCmd(SaveLane::Autosave)));
                Ok(())
            })?,
        )?;
        save_tbl.set(
            lua_save::LOAD_LATEST,
            lua.create_function(|_, ()| {
                push_command(Box::new(LoadLatestSaveCmd));
                Ok(())
            })?,
        )?;
        save_tbl.set(
            lua_save::REGISTER_PROVIDER,
            lua.create_function(register_provider)?,
        )?;
        save_tbl.set(
            lua_save::TO_STRING,
            lua.create_function(|_lua, value: mlua::Value| {
                let json = serde_json::to_string(&value).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to serialize: {e}"))
                })?;
                Ok(json)
            })?,
        )?;
        save_tbl.set(
            lua_save::FROM_STRING,
            lua.create_function(|lua, json: String| {
                let serde_val: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to deserialize: {e}"))
                })?;
                json_to_lua(lua, &serde_val)
            })?,
        )?;
        save_tbl.set(
            lua_save::HAS_LATEST,
            lua.create_function(|_, ()| {
                Ok(crate::engine::SaveRuntime::has_latest_save())
            })?,
        )?;
        engine_tbl.set(
            lua_engine::QUIT_TO_TITLE,
            lua.create_function(|lua, ()| {
                let ctx = LuaSaveCtx::borrow_ctx(lua)?;
                ctx.pending_quit_to_title.set(true);
                Ok(())
            })?,
        )?;

        engine_tbl.set(lua_save::SAVE, save_tbl)?;
        Ok(())
    }
}

impl LuaApi for SaveModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Runtime save/load system.");
        out.line(&format!("engine.{} = {{}}", lua_save::SAVE));
        out.line("");
        out.line("--- Save the current game state to the manual save lane.");
        out.line("---@return nil");
        out.line(&format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::MANUAL));
        out.line("");
        out.line("--- Save the current game state to the autosave lane.");
        out.line("---@return nil");
        out.line(&format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::AUTO));
        out.line("");
        out.line("--- Save a checkpoint (stored in the autosave lane).");
        out.line("---@return nil");
        out.line(&format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::CHECKPOINT));
        out.line("");
        out.line("--- Request loading the latest available runtime save.");
        out.line("---@return nil");
        out.line(&format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::LOAD_LATEST));
        out.line("");
        out.line("--- Register a save provider.");
        out.line("---@return nil");
        out.line(&format!(
            "---@param def table A table with `{}`, `{}`, `{}`, and `{}` fields.",
            lua_fields::ID, lua_save::PROVIDER_VERSION, lua_save::PROVIDER_CAPTURE, lua_save::PROVIDER_APPLY
        ));
        out.line(&format!(
            "function engine.{}.{}(def) end",
            lua_save::SAVE, lua_save::REGISTER_PROVIDER
        ));
        out.line("");
        out.line("--- Serialize a Lua value to a string.");
        out.line("---@return string");
        out.line(&format!(
            "function engine.{}.{}(value) end",
            lua_save::SAVE, lua_save::TO_STRING
        ));
        out.line("");
        out.line("--- Deserialize a string to a Lua value.");
        out.line("---@return table|nil");
        out.line(&format!(
            "function engine.{}.{}(json) end",
            lua_save::SAVE, lua_save::FROM_STRING
        ));
        out.line("");
        out.line("--- Returns true if a latest save exists on disk.");
        out.line("---@return boolean");
        out.line(&format!(
            "function engine.{}.{}() end",
            lua_save::SAVE, lua_save::HAS_LATEST
        ));
        out.line("");
    }
}

fn json_to_lua(lua: &Lua, value: &serde_json::Value) -> mlua::Result<mlua::Value> {
    match value {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(mlua::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(mlua::Value::Number(f))
            } else {
                Ok(mlua::Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i as i64 + 1, json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj {
                table.set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
    }
}
