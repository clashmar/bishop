use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::omni_error;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::{lua_event_tag, lua_events};
use engine_core::worlds::*;
use mlua::{Lua, Value, Variadic};

/// Selects the destination world.
#[derive(Debug, Clone)]
pub enum WorldSelector {
    ByName(String),
    ById(WorldId),
    Return,
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
        if matches!(request.world, WorldSelector::Return) {
            return Self::execute_return(lua, game_instance);
        }
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
            WorldTransitionMode::Overlay => {
                push_activation_frame(game, destination.world_id);
                switch_to_overlay(lua, game, &destination);
                true
            }
        }
    }

    /// Pops the activation stack and transitions back to the previous world; returns false when the stack is empty.
    pub fn execute_return(lua: &Lua, game_instance: &mut GameInstance) -> bool {
        let game = &mut game_instance.game;
        let Some(frame) = game.overlay_stack.pop() else {
            omni_error!(
                "return_from_world() called with empty overlay stack: \
                 the world must be entered via a WorldExit targeting an overlay world"
            );
            return false;
        };
        let Some(_world) = game.get_world(frame.world) else {
            omni_error!("return_from_world: world {:?} no longer exists", frame.world);
            return false;
        };
        let destination = Destination {
            world_id: frame.world,
            room_id: frame.room,
            position: Vec2::ZERO,
        };
        switch_to_overlay(lua, game, &destination);
        true
    }

    /// Executes the transition described by a `WorldExit` component on the given entity.
    pub fn execute_from_exit(lua: &Lua, game_instance: &mut GameInstance, entity: Entity) -> bool {
        let game = &game_instance.game;
        let Some(exit) = game.ecs.get::<WorldExit>(entity).cloned() else {
            return false;
        };
        match exit.destination {
            Some(ExitDestination::Return) => Self::execute_return(lua, game_instance),
            Some(ExitDestination::World(world_id)) => {
                let is_overlay = game_instance.game.get_world(world_id)
                    .is_some_and(|w| w.overlay);
                let mode = if is_overlay { WorldTransitionMode::Overlay } else { WorldTransitionMode::Transport };
                Self::execute(lua, game_instance, &WorldTransitionRequest {
                    entity: if is_overlay { None } else {
                        game_instance.game.ecs.get_player_entity()
                    },
                    world: WorldSelector::ById(world_id),
                    entry_name: exit.entry.clone(),
                    mode,
                })
            }
            None => false,
        }
    }
}

fn push_activation_frame(game: &mut Game, destination_world_id: WorldId) {
    let current_world_id = game.current_world().id;
    if current_world_id == destination_world_id {
        return;
    }
    let room = game.current_world().current_room_id.unwrap_or_default();
    game.overlay_stack.push(OverlayFrame { world: current_world_id, room });
}

fn resolve_destination(game: &Game, request: &WorldTransitionRequest) -> Option<Destination> {
    let world = match &request.world {
        WorldSelector::ByName(name) => game.worlds().iter().find(|w| &w.name == name),
        WorldSelector::ById(id) => game.get_world(*id),
        WorldSelector::Return => unreachable!("Return is handled before resolve_destination"),
    };
    let Some(world) = world else {
        omni_error!("Unknown world {:?}", request.world);
        return None;
    };

    match &request.entry_name {
        Some(entry_name) => resolve_entry(game, world, entry_name),
        None => resolve_world_start(game, world),
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

fn resolve_world_start(game: &Game, world: &World) -> Option<Destination> {
    let entries = game.ecs.get_store::<WorldEntry>();
    for (&entity, entry) in entries.data.iter() {
        if entry.name != WorldEntry::START {
            continue;
        }
        let Some(room_id) = game.ecs.get::<CurrentRoom>(entity).map(|r| r.0) else {
            continue;
        };
        if world.get_room(room_id).is_none() {
            continue;
        }
        let position = game
            .ecs
            .get::<Transform>(entity)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);
        return Some(Destination {
            world_id: world.id,
            room_id,
            position,
        });
    }

    // Fall back to first room at origin
    let room_id = world.rooms().first()?.id;
    Some(Destination {
        world_id: world.id,
        room_id,
        position: Vec2::ZERO,
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
        switch_to_overlay(lua, game, destination);
    }
    true
}

fn switch_to_overlay(lua: &Lua, game: &mut Game, destination: &Destination) {
    game.select_world(destination.world_id);
    if let Some(world) = game.current_world_mut() {
        world.current_room_id = Some(destination.room_id);
    }
    emit_world_entered(lua, game, destination.world_id);
}

fn emit_world_entered(lua: &Lua, game: &Game, world_id: WorldId) {
    let world = game.current_world();
    let mut args = vec![Value::Integer(world_id.0 as i64)];
    if let Ok(name) = lua.create_string(&world.name) {
        args.push(Value::String(name));
    }
    if let Ok(tags_table) = lua.create_table() {
        for tag in &world.tags {
            let tag_str = match tag {
                EventTag::Autosave => lua_event_tag::AUTOSAVE,
                EventTag::Custom(s) => s.as_str(),
            };
            let _ = tags_table.push(tag_str);
        }
        args.push(Value::Table(tags_table));
    }
    game.script_manager.event_bus.emit(
        lua_events::WORLD_ENTERED.to_string(),
        Variadic::from_iter(args),
    );
}
