use crate::engine::Engine;
use crate::engine::SaveRuntime;
use crate::save_system::SaveLane;
use crate::scripting::commands::lua_command::LuaCommand;

/// Queues a save to the specified lane.
pub struct SaveToLaneCmd(pub SaveLane);

impl LuaCommand for SaveToLaneCmd {
    fn execute(&mut self, engine: &mut Engine) {
        if let Err(err) = engine
            .save_runtime
            .save_to_lane(&engine.game_instance, self.0)
        {
            engine_core::onscreen_error!("Save to {:?} failed: {}", self.0, err);
        }
    }
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
