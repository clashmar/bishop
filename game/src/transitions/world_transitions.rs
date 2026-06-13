use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::omni_error;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::worlds::*;
use mlua::{Lua, Value, Variadic};

/// Selects the destination world by display name (Lua string API) or stable id (WorldExit component).
#[derive(Debug, Clone)]
pub enum WorldSelector {
    ByName(String),
    ById(WorldId),
}

/// An explicit, one-shot request to transition between worlds.
#[derive(Debug, Clone)]
pub struct WorldTransitionRequest {
    /// Subject entity. Required for `Transport`; ignored for `Activate`.
    pub entity: Option<Entity>,
    /// Destination world.
    pub world: WorldSelector,
    /// Destination `WorldEntry` name. `None` uses the world's start.
    pub entry_name: Option<String>,
    /// Transition mode.
    pub mode: WorldTransitionMode,
}

/// Resolved arrival point inside the destination world.
struct Destination {
    world_id: WorldId,
    room_id: RoomId,
    position: Vec2,
}

/// Executes explicit transitions between worlds.
pub struct WorldTransitionManager;

impl WorldTransitionManager {
    /// Executes a world transition; returns false when the request cannot be resolved.
    pub fn execute(
        lua: &Lua,
        game_instance: &mut GameInstance,
        request: &WorldTransitionRequest,
    ) -> bool {
        let game = &mut game_instance.game;
        let Some(destination) = resolve_destination(game, request) else {
            return false;
        };

        match request.mode {
            WorldTransitionMode::Transport => {
                let Some(entity) = request.entity else {
                    omni_error!("World transport requires a subject entity");
                    return false;
                };
                transport(lua, game, entity, &destination)
            }
            WorldTransitionMode::Activate => {
                activate(lua, game, &destination);
                true
            }
        }
    }
}

fn resolve_destination(game: &Game, request: &WorldTransitionRequest) -> Option<Destination> {
    let world = match &request.world {
        WorldSelector::ByName(name) => game.worlds().iter().find(|w| &w.name == name),
        WorldSelector::ById(id) => game.get_world(*id),
    };
    let Some(world) = world else {
        omni_error!("Unknown world {:?}", request.world);
        return None;
    };

    match &request.entry_name {
        Some(entry_name) => resolve_entry(game, world, entry_name),
        None => resolve_world_start(world),
    }
}

fn resolve_entry(game: &Game, world: &World, entry_name: &str) -> Option<Destination> {
    let entries = game.ecs.get_store::<WorldEntry>();

    for (&entity, entry) in entries.data.iter() {
        if entry.name != entry_name {
            continue;
        }
        let Some(room) = game.ecs.get::<CurrentRoom>(entity).map(|room| room.0) else {
            continue;
        };
        if world.get_room(room).is_none() {
            continue;
        }
        let Some(position) = game
            .ecs
            .get::<Transform>(entity)
            .map(|transform| transform.position)
        else {
            omni_error!("Entry '{}' in world '{}' has no Transform", entry_name, world.name);
            return None;
        };
        return Some(Destination {
            world_id: world.id,
            room_id: room,
            position,
        });
    }

    omni_error!("No entry '{}' in world '{}'", entry_name, world.name);
    None
}

fn resolve_world_start(world: &World) -> Option<Destination> {
    let Some(room_id) = world.starting_room_id else {
        omni_error!(
            "World '{}' has no starting room and no entry was given",
            world.name
        );
        return None;
    };
    Some(Destination {
        world_id: world.id,
        room_id,
        position: world.starting_position.unwrap_or(Vec2::ZERO),
    })
}

fn transport(lua: &Lua, game: &mut Game, entity: Entity, destination: &Destination) -> bool {
    let Some(transform) = game.ecs.get_mut::<Transform>(entity) else {
        omni_error!("World transport subject {entity:?} has no Transform");
        return false;
    };
    transform.position = destination.position;
    game.ecs.set_current_room(entity, destination.room_id);

    if game.ecs.get_player_entity() == Some(entity) {
        activate(lua, game, destination);
    }
    true
}

fn activate(lua: &Lua, game: &mut Game, destination: &Destination) {
    game.select_world(destination.world_id);
    if let Some(world) = game.current_world_mut() {
        world.current_room_id = Some(destination.room_id);
    }
    emit_world_entered(lua, game, destination.world_id);
}

fn emit_world_entered(lua: &Lua, game: &Game, world_id: WorldId) {
    let mut args = vec![Value::Integer(world_id.0 as i64)];
    if let Ok(name) = lua.create_string(&game.current_world().name) {
        args.push(Value::String(name));
    }
    game.script_manager.event_bus.emit(
        lua_events::WORLD_ENTERED.to_string(),
        Variadic::from_iter(args),
    );
}
