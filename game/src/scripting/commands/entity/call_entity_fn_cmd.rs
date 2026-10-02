use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::interact::handle_interactions;
use engine_core::ecs::{Entity, Script};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::omni_error;
use mlua::{Function, MultiValue, Table, Value};

/// Calls a function on an entity.
pub struct CallEntityFnCmd {
    pub entity: Entity,
    pub fn_name: String,
    pub args: Vec<Value>,
}

impl LuaCommand for CallEntityFnCmd {
    fn execute(&mut self, engine: &mut Engine) {
        {
            let game_instance = engine.game_instance.borrow();
            let instance_and_func =
                lookup_entity_function(&game_instance.game, self.entity, &self.fn_name);

            if let Some((instance, func)) = instance_and_func {
                if let Err(e) = call_entity_function(instance, func, &self.args) {
                    omni_error!("Lua call failed: {}", e);
                }
            }
        }

        if self.fn_name == lua_entity::INTERACT {
            let mut game_instance = engine.game_instance.borrow_mut();
            handle_interactions(self.entity, &mut game_instance);
        }
    }
}

pub(crate) fn lookup_entity_function(
    game: &Game,
    entity: Entity,
    fn_name: &str,
) -> Option<(Table, Function)> {
    let script = game.ecs.get::<Script>(entity)?;
    let instance = game.script_manager.instances.get(&(entity, script.script_id))?;
    let func = instance.get::<Function>(fn_name).ok()?;
    Some((instance.clone(), func))
}

fn call_entity_function(instance: Table, func: Function, args: &[Value]) -> mlua::Result<()> {
    let handle = Value::Table(instance);
    let mut call_args = Vec::with_capacity(args.len() + 1);
    call_args.push(handle);
    call_args.extend(args.iter().cloned());
    func.call::<()>(MultiValue::from_vec(call_args))
}
