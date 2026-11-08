use crate::engine::Engine;
use crate::save_system::{SaveLane, SaveWrite};
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::scripting::EventBus;
use mlua::{Lua, Value, Variadic};

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
            Ok(SaveWrite::Committed) => {
                emit_save_succeeded(&engine.lua, &event_bus, self.lane, &self.trigger);
            }
            Ok(SaveWrite::Pending(id)) => {
                engine.save_runtime.track_pending(id, self.lane, std::mem::take(&mut self.trigger));
            }
            Err(err) => {
                emit_save_failed(&engine.lua, &event_bus, self.lane, &self.trigger, &err.to_string());
            }
        }
    }
}

/// Emits success after a native write or committed browser transaction.
pub(crate) fn emit_save_succeeded(lua: &Lua, event_bus: &EventBus, lane: SaveLane, trigger: &str) {
    if let Some(args) = save_event_args(lua, lane.file_stem(), trigger) {
        event_bus.emit(lua_events::SAVE_SUCCEEDED.to_string(), args);
    }
}

/// Emits a failed save with its lane, trigger and error.
pub(crate) fn emit_save_failed(
    lua: &Lua, event_bus: &EventBus, lane: SaveLane, trigger: &str, error: &str,
) {
    engine_core::omni_error!("Save to {:?} failed: {}", lane, error);
    if let Some(args) = save_failed_event_args(lua, lane.file_stem(), trigger, error) {
        event_bus.emit(lua_events::SAVE_FAILED.to_string(), args);
    }
}

fn save_event_args(lua: &Lua, lane: &str, trigger: &str) -> Option<Variadic<Value>> {
    let lane = lua.create_string(lane).ok()?;
    let trigger = lua.create_string(trigger).ok()?;
    Some(Variadic::from_iter([
        Value::String(lane),
        Value::String(trigger),
    ]))
}

fn save_failed_event_args(
    lua: &Lua,
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
        if engine.save_runtime.has_latest_save() {
            engine.save_runtime.request_latest_runtime_load();
        }
    }
}
