use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::scripting::script::Script;
use engine_core::*;
use mlua::{Function, MultiValue, Value};

/// Calls a function on an entity.
pub struct CallEntityFnCmd {
    pub entity: Entity,
    pub fn_name: String,
    pub args: Vec<Value>,
}

impl LuaCommand for CallEntityFnCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let game_instance = engine.game_instance.borrow();
        let ecs = &game_instance.game.ecs;

        let script = match ecs.get::<Script>(self.entity) {
            Some(s) => s,
            None => return,
        };

        let instance = match game_instance
            .game
            .script_manager
            .instances
            .get(&(self.entity, script.script_id))
        {
            Some(t) => t,
            None => return,
        };

        let Ok(func) = instance.get::<Function>(&*self.fn_name) else {
            return;
        };

        let handle = Value::Table(instance.clone());

        let mut call_args = Vec::with_capacity(self.args.len() + 1);
        call_args.push(handle);
        call_args.extend(self.args.clone());

        if let Err(e) = func.call::<()>(MultiValue::from_vec(call_args)) {
            onscreen_error!("Lua call failed: {}", e);
        }
    }
}
