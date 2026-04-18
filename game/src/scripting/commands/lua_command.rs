// game/src/scripting/commands/lua_command.rs
use crate::engine::game_instance::GameInstance;
use crate::engine::Engine;
use bishop::prelude::Vec2;
use engine_core::animation::animation_clip::*;
use engine_core::ecs::component_registry::public_lua_component;
use engine_core::ecs::ecs::Ecs;
use engine_core::ecs::entity::{get_children, Entity};
use engine_core::ecs::facing_direction::*;
use engine_core::ecs::transform::{update_entity_position, Transform};
use engine_core::prelude::SubPixel;
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

pub(crate) fn reposition_entity(
    game_instance: &mut GameInstance,
    entity: Entity,
    target_position: Vec2,
) {
    let moved_entities = collect_subtree_entities(&game_instance.game.ecs, entity);
    update_entity_position(&mut game_instance.game.ecs, entity, target_position);

    for moved_entity in moved_entities {
        if let Some(sub_pixel) = game_instance.game.ecs.get_mut::<SubPixel>(moved_entity) {
            sub_pixel.x = 0.0;
            sub_pixel.y = 0.0;
        }

        if let Some(position) = game_instance
            .game
            .ecs
            .get::<Transform>(moved_entity)
            .map(|transform| transform.position)
        {
            game_instance.prev_positions.insert(moved_entity, position);
        }
    }
}

fn collect_subtree_entities(ecs: &Ecs, entity: Entity) -> Vec<Entity> {
    let mut entities = vec![entity];

    for child in get_children(ecs, entity) {
        entities.extend(collect_subtree_entities(ecs, child));
    }

    entities
}

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
    use engine_core::ecs::component::comp_type_name;
    use engine_core::prelude::PrefabInstanceRoot;
    use engine_core::prelude::*;

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
    fn reposition_entity_moves_entity_clears_subpixel_and_preserves_velocity() {
        let mut game_instance = crate::engine::game_instance::GameInstance {
            game: Game::default(),
            prev_positions: std::collections::HashMap::new(),
        };
        let entity = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(4.0, 5.0),
                ..Default::default()
            })
            .with(Velocity { x: 3.0, y: -2.0 })
            .with(SubPixel { x: 0.25, y: -0.5 })
            .finish();
        game_instance
            .prev_positions
            .insert(entity, Vec2::new(-8.0, 9.0));

        reposition_entity(&mut game_instance, entity, Vec2::new(22.0, 31.0));

        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(22.0, 31.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Velocity>(entity)
                .map(|velocity| (velocity.x, velocity.y)),
            Some((3.0, -2.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<SubPixel>(entity)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            game_instance.prev_positions.get(&entity).copied(),
            Some(Vec2::new(22.0, 31.0))
        );
    }

    #[test]
    fn reposition_entity_moves_children_with_parent() {
        let mut game_instance = crate::engine::game_instance::GameInstance {
            game: Game::default(),
            prev_positions: std::collections::HashMap::new(),
        };
        let parent = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .finish();
        let child = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(13.0, 14.0),
                ..Default::default()
            })
            .with(SubPixel { x: 0.5, y: -0.25 })
            .finish();
        set_parent(&mut game_instance.game.ecs, child, parent);
        game_instance
            .prev_positions
            .insert(parent, Vec2::new(10.0, 12.0));
        game_instance
            .prev_positions
            .insert(child, Vec2::new(13.0, 14.0));

        reposition_entity(&mut game_instance, parent, Vec2::new(20.0, 30.0));

        let child = game_instance
            .game
            .ecs
            .get::<Children>(parent)
            .and_then(|children| children.entities.first().copied())
            .unwrap();
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(23.0, 32.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<SubPixel>(child)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            game_instance.prev_positions.get(&child).copied(),
            Some(Vec2::new(23.0, 32.0))
        );
    }
}
