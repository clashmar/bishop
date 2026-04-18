use crate::engine::Engine;
use crate::scripting::commands::entity::flip_x_for_direction;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::facing_direction::{Direction, FacingDirection};
use engine_core::prelude::Animation;

/// Sets the facing direction on an entity.
pub struct SetFacingCmd {
    pub entity: Entity,
    pub direction: Direction,
}

impl LuaCommand for SetFacingCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        ecs.add_component_to_entity(self.entity, FacingDirection(self.direction));

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            if let Some(current_id) = &animation.current {
                if let Some(clip) = animation.clips.get(current_id) {
                    if clip.mirrored {
                        animation.flip_x = flip_x_for_direction(self.direction);
                    }
                }
            }
        }
    }
}
