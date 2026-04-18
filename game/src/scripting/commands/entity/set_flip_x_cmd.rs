use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::Animation;

/// Sets the horizontal flip state on an entity's animation.
pub struct SetFlipXCmd {
    pub entity: Entity,
    pub flip_x: bool,
}

impl LuaCommand for SetFlipXCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            animation.flip_x = self.flip_x;
        }
    }
}
