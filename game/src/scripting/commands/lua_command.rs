// game/src/scripting/commands/lua_command.rs
use crate::engine::Engine;
use engine_core::animation::animation_clip::*;
use engine_core::ecs::component_registry::COMPONENTS;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::facing_direction::*;
use engine_core::scripting::script::Script;
use engine_core::*;
use mlua::Function;
use mlua::MultiValue;
use mlua::Value;

/// All mutating Lua actions implement this.
pub trait LuaCommand {
    /// Execute the command, mutating the supplied `GameState`.
    fn execute(&mut self, engine: &mut Engine);
}

/// Set a component on an entity.
pub struct SetComponentCmd {
    pub entity: usize,
    pub comp_name: String,
    pub value: Value,
}

impl LuaCommand for SetComponentCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == self.comp_name) {
            if let Ok(boxed) = (reg.from_lua)(&engine.lua, self.value.clone()) {
                (reg.inserter)(&mut game_instance.game.ecs, Entity(self.entity), boxed);
            } else {
                onscreen_error!("Failed to convert value for component '{}'", self.comp_name);
            }
        } else {
            onscreen_error!("Unknown component '{}'", self.comp_name);
        }
    }
}

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

        let script = match ecs.get::<Script>(self.entity) {
            Some(s) => s,
            None => return,
        };

        let instance = match game_instance
            .game
            .script_manager
            .instances
            .get(&(self.entity, script.script_id))
        {
            Some(t) => t,
            None => return,
        };

        let Ok(func) = instance.get::<Function>(&*self.fn_name) else {
            return;
        };

        let handle = Value::Table(instance.clone());

        let mut call_args = Vec::with_capacity(self.args.len() + 1);
        call_args.push(handle);
        call_args.extend(self.args.clone());

        if let Err(e) = func.call::<()>(MultiValue::from_vec(call_args)) {
            onscreen_error!("Lua call failed: {}", e);
        }
    }
}

/// Sets the active animation clip on an entity.
pub struct SetClipCmd {
    pub entity: Entity,
    pub clip_name: String,
}

impl LuaCommand for SetClipCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;

        // Get facing direction first (before mutable borrow of Animation)
        let facing_left = ecs
            .get::<FacingDirection>(self.entity)
            .map(|f| flip_x_for_direction(f.0))
            .unwrap_or(false);

        if let Some(animation) = ecs.get_mut::<Animation>(self.entity) {
            let clip_id = string_to_clip_id(&self.clip_name);
            animation.set_clip(&clip_id);

            // Recalculate flip_x based on new clip's mirrored property
            if let Some(clip) = animation.clips.get(&clip_id) {
                animation.flip_x = clip.mirrored && facing_left;
            }
        }
    }
}

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

        // Auto-flip if current clip has mirrored enabled
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

/// Converts a string clip name to a ClipId.
fn string_to_clip_id(name: &str) -> ClipId {
    match name.to_lowercase().as_str() {
        "idle" => ClipId::Idle,
        "walk" => ClipId::Walk,
        "run" => ClipId::Run,
        "attack" => ClipId::Attack,
        "jump" => ClipId::Jump,
        "fall" => ClipId::Fall,
        _ => ClipId::Custom(name.to_string()),
    }
}

pub(crate) fn parse_direction(value: &str) -> Result<Direction, String> {
    match value.trim().to_lowercase().as_str() {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        "left" => Ok(Direction::Left),
        "right" => Ok(Direction::Right),
        "up_left" | "upleft" => Ok(Direction::UpLeft),
        "up_right" | "upright" => Ok(Direction::UpRight),
        "down_left" | "downleft" => Ok(Direction::DownLeft),
        "down_right" | "downright" => Ok(Direction::DownRight),
        other => Err(format!(
            "Unsupported direction '{other}'. Expected one of: up, down, left, right, up_left, up_right, down_left, down_right."
        )),
    }
}

pub(crate) fn flip_x_for_direction(direction: Direction) -> bool {
    direction.has_leftward_component()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_direction_accepts_legacy_and_new_direction_strings() {
        assert_eq!(parse_direction("left").unwrap(), Direction::Left);
        assert_eq!(parse_direction("up_left").unwrap(), Direction::UpLeft);
        assert_eq!(parse_direction("DownRight").unwrap(), Direction::DownRight);
    }

    #[test]
    fn parse_direction_rejects_unknown_values() {
        assert!(parse_direction("north").is_err());
    }

    #[test]
    fn leftward_flip_helper_only_flips_for_leftward_directions() {
        assert!(flip_x_for_direction(Direction::Left));
        assert!(flip_x_for_direction(Direction::DownLeft));
        assert!(!flip_x_for_direction(Direction::Up));
        assert!(!flip_x_for_direction(Direction::Right));
    }
}
