use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::omni_error;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::{lua_event_tag, lua_events};
use engine_core::worlds::*;
use mlua::prelude::LuaResult;
use mlua::{Lua, LuaSerdeExt, Table, Value, Variadic};
use serde::Deserialize;

/// Selects a world destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldSelector {
    /// Select by authored world name.
    ByName(String),
    /// Select by stable world id.
    ById(WorldId),
}

/// Typed data carried by generated Lua entry handles.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryHandleData {
    /// Destination world id.
    pub world_id: WorldId,
    /// Destination room id.
    pub room_id: RoomId,
    /// Authored entry name.
    pub entry_name: String,
    /// Optional override x position.
    pub x: Option<f32>,
    /// Optional override y position.
    pub y: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LuaEntryHandleData {
    world_id: usize,
    room_id: usize,
    entry_name: String,
    x: Option<f32>,
    y: Option<f32>,
}

impl EntryHandleData {
    /// Parses a typed generated Lua entry handle table.
    pub fn from_lua(lua: &Lua, entry: Table) -> LuaResult<Self> {
        let entry: LuaEntryHandleData = lua.from_value(Value::Table(entry))?;
        Ok(Self {
            world_id: WorldId(entry.world_id),
            room_id: RoomId(entry.room_id),
            entry_name: entry.entry_name,
            x: entry.x,
            y: entry.y,
        })
    }

    fn position(&self) -> Option<Vec2> {
        self.x.zip(self.y).map(|(x, y)| Vec2::new(x, y))
    }
}

/// Canonical traversal destination kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum DestinationSelector {
    /// Move or overlay to a typed entry handle destination.
    Entry(EntryHandleData),
    /// Move or overlay to a world, optionally targeting a named entry.
    World {
        /// World selection strategy.
        selector: WorldSelector,
        /// Optional authored entry name.
        entry_name: Option<String>,
    },
    /// Restore a concrete world, room, and position.
    RestoreLocation {
        /// Destination world id.
        world_id: WorldId,
        /// Destination room id.
        room_id: RoomId,
        /// Destination x position.
        x: f32,
        /// Destination y position.
        y: f32,
    },
    /// Pop the overlay stack and return to the caller.
    Return,
}

/// Canonical request queued by scripted/world-exit traversal APIs.
#[derive(Debug, Clone, PartialEq)]
pub struct TraversalRequest {
    /// Subject entity for transport-style traversal.
    pub entity: Option<Entity>,
    /// Canonical destination selector.
    pub destination: DestinationSelector,
    /// Traversal mode.
    pub mode: WorldTransitionMode,
}

impl TraversalRequest {
    /// Builds a request targeting a world destination.
    pub fn to_world(
        entity: Option<Entity>,
        selector: WorldSelector,
        entry_name: Option<String>,
        mode: WorldTransitionMode,
    ) -> Self {
        Self {
            entity,
            destination: DestinationSelector::World {
                selector,
                entry_name,
            },
            mode,
        }
    }

    /// Builds a transport request for an entity.
    pub fn transport(entity: Entity, selector: WorldSelector, entry_name: Option<String>) -> Self {
        Self::to_world(Some(entity), selector, entry_name, WorldTransitionMode::Transport)
    }

    /// Builds an overlay-world request.
    pub fn overlay_world(selector: WorldSelector, entry_name: Option<String>) -> Self {
        Self::to_world(None, selector, entry_name, WorldTransitionMode::Overlay)
    }

    /// Builds an overlay-entry request.
    pub fn overlay_entry(entry: EntryHandleData) -> Self {
        Self {
            entity: None,
            destination: DestinationSelector::Entry(entry),
            mode: WorldTransitionMode::Overlay,
        }
    }

    /// Builds a return-from-world request.
    pub fn return_from_world() -> Self {
        Self {
            entity: None,
            destination: DestinationSelector::Return,
            mode: WorldTransitionMode::Overlay,
        }
    }

    /// Builds a concrete restore-location request.
    pub fn restore_location(
        entity: Entity,
        world_id: WorldId,
        room_id: RoomId,
        position: Vec2,
    ) -> Self {
        Self {
            entity: Some(entity),
            destination: DestinationSelector::RestoreLocation {
                world_id,
                room_id,
                x: position.x,
                y: position.y,
            },
            mode: WorldTransitionMode::Transport,
        }
    }
}

/// Resolved arrival point inside the destination world.
struct Destination {
    world_id: WorldId,
    room_id: RoomId,
    position: Vec2,
}

/// Executes canonical traversal requests.
pub struct WorldTransitionManager;

impl WorldTransitionManager {
    /// Executes a traversal request; returns false when the request cannot be resolved.
    pub fn execute(lua: &Lua, game_instance: &mut GameInstance, request: &TraversalRequest) -> bool {
        if matches!(request.destination, DestinationSelector::Return) {
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

    /// Pops the overlay stack and transitions back to the previous world; returns false when the stack is empty.
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

    /// Executes the traversal described by a `WorldExit` component on the given entity.
    pub fn execute_from_exit(lua: &Lua, game_instance: &mut GameInstance, entity: Entity) -> bool {
        let game = &game_instance.game;
        let Some(exit) = game.ecs.get::<WorldExit>(entity).cloned() else {
            return false;
        };
        match exit.destination {
            Some(ExitDestination::Return) => Self::execute_return(lua, game_instance),
            Some(ExitDestination::World(world_id)) => {
                let is_overlay = game_instance
                    .game
                    .get_world(world_id)
                    .is_some_and(|world| world.overlay);
                let request = if is_overlay {
                    TraversalRequest::overlay_world(WorldSelector::ById(world_id), exit.entry.clone())
                } else {
                    let Some(player) = game_instance.game.ecs.get_player_entity() else {
                        omni_error!("World transport requires a player entity");
                        return false;
                    };
                    TraversalRequest::transport(player, WorldSelector::ById(world_id), exit.entry.clone())
                };
                Self::execute(lua, game_instance, &request)
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
    game.overlay_stack.push(OverlayFrame {
        world: current_world_id,
        room,
    });
}

fn resolve_destination(game: &Game, request: &TraversalRequest) -> Option<Destination> {
    match &request.destination {
        DestinationSelector::Entry(entry) => resolve_entry_handle_destination(game, entry),
        DestinationSelector::World {
            selector,
            entry_name,
        } => resolve_world_destination(game, selector, entry_name.as_deref()),
        DestinationSelector::RestoreLocation {
            world_id,
            room_id,
            x,
            y,
        } => resolve_restore_location_destination(game, *world_id, *room_id, *x, *y),
        DestinationSelector::Return => None,
    }
}

fn resolve_entry_handle_destination(game: &Game, entry: &EntryHandleData) -> Option<Destination> {
    let Some(world) = game.get_world(entry.world_id) else {
        omni_error!("Unknown world id {:?} for entry handle", entry.world_id);
        return None;
    };

    if let Some(position) = entry.position() {
        if world.get_room(entry.room_id).is_none() {
            omni_error!(
                "Entry handle points to missing room {:?} in world '{}'",
                entry.room_id,
                world.name
            );
            return None;
        }
        return Some(Destination {
            world_id: entry.world_id,
            room_id: entry.room_id,
            position,
        });
    }

    resolve_entry(game, world, &entry.entry_name).or_else(|| {
        world.get_room(entry.room_id).map(|_| Destination {
            world_id: entry.world_id,
            room_id: entry.room_id,
            position: Vec2::ZERO,
        })
    })
}

fn resolve_world_destination(
    game: &Game,
    selector: &WorldSelector,
    entry_name: Option<&str>,
) -> Option<Destination> {
    let world = match selector {
        WorldSelector::ByName(name) => game.worlds().iter().find(|world| &world.name == name),
        WorldSelector::ById(id) => game.get_world(*id),
    };
    let Some(world) = world else {
        omni_error!("Unknown world {:?}", selector);
        return None;
    };

    match entry_name {
        Some(entry_name) => resolve_entry(game, world, entry_name),
        None => resolve_world_start(game, world),
    }
}

fn resolve_restore_location_destination(
    game: &Game,
    world_id: WorldId,
    room_id: RoomId,
    x: f32,
    y: f32,
) -> Option<Destination> {
    let Some(world) = game.get_world(world_id) else {
        omni_error!("Unknown world id {:?} in restore_location", world_id);
        return None;
    };
    if world.get_room(room_id).is_none() {
        omni_error!(
            "Unknown room id {:?} in restore_location for world '{}'",
            room_id,
            world.name
        );
        return None;
    }
    Some(Destination {
        world_id,
        room_id,
        position: Vec2::new(x, y),
    })
}

fn resolve_entry(game: &Game, world: &World, entry_name: &str) -> Option<Destination> {
    let entries = game.ecs.get_store::<WorldEntry>();

    for (&entity, entry) in entries.data.iter() {
        if entry.name != entry_name {
            continue;
        }
        let Some(room) = game.ecs.get::<CurrentRoom>(entity).map(|current_room| current_room.0) else {
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
        let Some(room_id) = game.ecs.get::<CurrentRoom>(entity).map(|room| room.0) else {
            continue;
        };
        if world.get_room(room_id).is_none() {
            continue;
        }
        let position = game
            .ecs
            .get::<Transform>(entity)
            .map(|transform| transform.position)
            .unwrap_or(Vec2::ZERO);
        return Some(Destination {
            world_id: world.id,
            room_id,
            position,
        });
    }

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
                EventTag::Custom(tag) => tag.as_str(),
            };
            let _ = tags_table.push(tag_str);
        }
        args.push(Value::Table(tags_table));
    }
    game.script_manager
        .event_bus
        .emit(lua_events::WORLD_ENTERED.to_string(), Variadic::from_iter(args));
}
