use crate::engine::Engine;
use crate::engine::SaveRuntime;
use crate::save_system::SaveLane;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::scripting::lua_constants::lua_events;
use mlua::{Value, Variadic};

/// Queues a save to the specified lane with its trigger label.
pub struct SaveToLaneCmd {
    pub lane: SaveLane,
    pub trigger: String,
}

impl LuaCommand for SaveToLaneCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let event_bus = engine
            .game_instance
            .borrow()
            .game
            .script_manager
            .event_bus
            .clone();
        match engine
            .save_runtime
            .save_to_lane(&engine.game_instance, self.lane)
        {
            Ok(()) => {
                if let Some(args) = save_event_args(&engine.lua, self.lane.file_stem(), &self.trigger)
                {
                    event_bus.emit(lua_events::SAVE_SUCCEEDED.to_string(), args);
                }
            }
            Err(err) => {
                engine_core::omni_error!("Save to {:?} failed: {}", self.lane, err);
                if let Some(args) = save_failed_event_args(
                    &engine.lua,
                    self.lane.file_stem(),
                    &self.trigger,
                    &err.to_string(),
                ) {
                    event_bus.emit(lua_events::SAVE_FAILED.to_string(), args);
                }
            }
        }
    }
}

fn save_event_args(lua: &mlua::Lua, lane: &str, trigger: &str) -> Option<Variadic<Value>> {
    let lane = lua.create_string(lane).ok()?;
    let trigger = lua.create_string(trigger).ok()?;
    Some(Variadic::from_iter([
        Value::String(lane),
        Value::String(trigger),
    ]))
}

fn save_failed_event_args(
    lua: &mlua::Lua,
    lane: &str,
    trigger: &str,
    error: &str,
) -> Option<Variadic<Value>> {
    let lane = lua.create_string(lane).ok()?;
    let trigger = lua.create_string(trigger).ok()?;
    let error = lua.create_string(error).ok()?;
    Some(Variadic::from_iter([
        Value::String(lane),
        Value::String(trigger),
        Value::String(error),
    ]))
}

/// Requests a runtime load of the latest save.
pub struct LoadLatestSaveCmd;

impl LuaCommand for LoadLatestSaveCmd {
    fn execute(&mut self, engine: &mut Engine) {
        if SaveRuntime::has_latest_save() {
            engine.save_runtime.request_latest_runtime_load();
        }
    }
}
