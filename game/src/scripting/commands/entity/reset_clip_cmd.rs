use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::Animation;

/// Resets the current animation clip to frame 0.
pub struct ResetClipCmd {
    pub entity: Entity,
}

impl LuaCommand for ResetClipCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            if let Some(current_id) = &animation.current.clone() {
                if let Some(state) = animation.states.get_mut(current_id) {
                    state.timer = 0.0;
                    state.col = 0;
                    state.row = 0;
                    state.finished = false;
                }
            }
        }
    }
}
