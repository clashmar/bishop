use crate::engine::Engine;
use crate::scripting::commands::entity::reposition_entity;
use crate::scripting::commands::lua_command::LuaCommand;
use bishop::prelude::Vec2;
use engine_core::ecs::entity::Entity;

/// Instantly repositions an entity without changing its velocity.
pub struct RepositionEntityCmd {
    pub entity: Entity,
    pub target_position: Vec2,
}

impl LuaCommand for RepositionEntityCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        reposition_entity(&mut game_instance, self.entity, self.target_position);
    }
}
