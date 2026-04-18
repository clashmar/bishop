use crate::engine::Engine;
use crate::scripting::commands::entity::reposition_entity;
use crate::scripting::commands::lua_command::LuaCommand;
use bishop::prelude::Vec2;
use engine_core::ecs::{Entity, Transform};

/// Offsets an entity by an immediate world-space delta.
pub struct MoveEntityByCmd {
    pub entity: Entity,
    pub delta: Vec2,
}

impl LuaCommand for MoveEntityByCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let current_position = game_instance
            .game
            .ecs
            .get::<Transform>(self.entity)
            .map(|transform| transform.position);

        if let Some(current_position) = current_position {
            reposition_entity(
                &mut game_instance,
                self.entity,
                current_position + self.delta,
            );
        }
    }
}
