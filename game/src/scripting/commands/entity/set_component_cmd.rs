use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::component_registry::public_lua_component;
use engine_core::ecs::entity::Entity;
use engine_core::*;
use mlua::Value;

/// Set a component on an entity.
pub struct SetComponentCmd {
    pub entity: usize,
    pub comp_name: String,
    pub value: Value,
}

impl LuaCommand for SetComponentCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        match public_lua_component(&self.comp_name) {
            Ok(reg) => {
                if let Ok(boxed) = (reg.from_lua)(&engine.lua, self.value.clone()) {
                    (reg.inserter)(&mut game_instance.game.ecs, Entity(self.entity), boxed);
                } else {
                    onscreen_error!("Failed to convert value for component '{}'", self.comp_name);
                }
            }
            Err(err) => onscreen_error!("{}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::component::comp_type_name;
    use engine_core::prelude::{CurrentRoom, PrefabInstanceRoot};

    #[test]
    fn set_component_command_rejects_private_components() {
        let type_name = comp_type_name::<PrefabInstanceRoot>();
        let err = match public_lua_component(type_name) {
            Ok(_) => panic!("private component should not be settable from Lua"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            format!("Component '{type_name}' is not available to Lua")
        );
    }

    #[test]
    fn set_component_command_rejects_current_room_after_it_becomes_private() {
        let type_name = comp_type_name::<CurrentRoom>();
        let err = match public_lua_component(type_name) {
            Ok(_) => panic!("CurrentRoom should be private to generic Lua APIs"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            format!("Component '{type_name}' is not available to Lua")
        );
    }
}
