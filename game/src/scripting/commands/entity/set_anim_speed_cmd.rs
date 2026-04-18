use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::Animation;

/// Sets the animation playback speed multiplier.
pub struct SetAnimSpeedCmd {
    pub entity: Entity,
    pub speed: f32,
}

impl LuaCommand for SetAnimSpeedCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            animation.speed_multiplier = self.speed.max(0.0);
        }
    }
}
