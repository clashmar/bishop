use crate::engine::Engine;
use crate::scripting::commands::entity::flip_x_for_direction;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::animation::animation_clip::ClipId;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::facing_direction::FacingDirection;
use engine_core::prelude::Animation;
use strum::IntoEnumIterator;

/// Sets the active animation clip on an entity.
pub struct SetClipCmd {
    pub entity: Entity,
    pub clip_name: String,
}

impl LuaCommand for SetClipCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        let facing_left = ecs
            .get::<FacingDirection>(self.entity)
            .map(|f| flip_x_for_direction(f.0))
            .unwrap_or(false);

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            let clip_id = string_to_clip_id(&self.clip_name);
            animation.set_clip(&clip_id);

            if let Some(clip) = animation.clips.get(&clip_id) {
                animation.flip_x = clip.mirrored && facing_left;
            }
        }
    }
}

fn string_to_clip_id(name: &str) -> ClipId {
    for clip_id in ClipId::iter() {
        if clip_id.to_string() == name {
            return clip_id;
        }
    }

    ClipId::Custom(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_to_clip_id_accepts_clip_display_strings() {
        let clip_ids = [
            ClipId::Idle,
            ClipId::Walk,
            ClipId::Run,
            ClipId::Attack,
            ClipId::Jump,
            ClipId::Fall,
            ClipId::Custom("Fidget".to_string()),
            ClipId::New,
        ];

        for clip_id in clip_ids {
            let label = clip_id.to_string();
            assert_eq!(string_to_clip_id(&label), clip_id);
        }
    }
}
