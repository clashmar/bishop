use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::interact::handle_interactions;
use engine_core::ecs::{Entity, Script};
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::{omni_error};
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

        let script = ecs.get::<Script>(self.entity);
        let instance_and_func = script.and_then(|s| {
            let instance = game_instance
                .game
                .script_manager
                .instances
                .get(&(self.entity, s.script_id))?;
            let func = instance.get::<Function>(&*self.fn_name).ok()?;
            Some((instance.clone(), func))
        });

        if let Some((instance, func)) = instance_and_func {
            let handle = Value::Table(instance);
            let mut call_args = Vec::with_capacity(self.args.len() + 1);
            call_args.push(handle);
            call_args.extend(self.args.clone());
            if let Err(e) = func.call::<()>(MultiValue::from_vec(call_args)) {
                omni_error!("Lua call failed: {}", e);
            }
        }

        if self.fn_name == lua_entity::INTERACT {
            handle_interactions(self.entity, &game_instance);
        }
    }
}
