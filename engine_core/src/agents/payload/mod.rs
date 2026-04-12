use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::{comp_type_name, CurrentRoom, Name};
use crate::ecs::component_registry::ComponentRegistry;
use crate::ecs::entity::Entity;
use crate::ecs::transform::Transform;
use crate::game::startup_mode::StartupMode;
use crate::game::Game;
use crate::prefab::PrefabLibrary;
use crate::scripting::script::Script;
use crate::worlds::room::{Room, RoomId};
use crate::worlds::world::{World, WorldId, WorldMeta};
use mlua::Lua;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Typed payload model used to assemble playtest sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentPayloadSpec {
    pub game_name: String,
    pub mode: AgentPayloadMode,
    worlds: Vec<AgentWorldSpec>,
}

/// Source of a payload.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentPayloadMode {
    Seeded,
    Synthetic,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentWorldSpec {
    name: String,
    rooms: Vec<AgentRoomSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentRoomSpec {
    name: String,
    entities: HashMap<String, AgentEntitySpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct AgentEntitySpec {
    components: Vec<AgentComponentSpec>,
    scripts: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentComponentSpec {
    entity: String,
    type_name: String,
    ron: String,
}

/// Errors returned while assembling a payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentPayloadError {
    UnknownComponentType(String),
    UnknownScriptType(String),
    MissingRoom,
    MissingEntity(String),
}

/// Final payload produced by the builder.
#[derive(Serialize, Deserialize)]
pub struct AgentBuiltPayload {
    pub game: Game,
    pub room: Room,
    pub startup_mode: StartupMode,
    pub spec: AgentPayloadSpec,
    pub entities: Vec<AgentEntityExport>,
}

/// A concrete exported entity used when materializing a payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEntityExport {
    pub entity: Entity,
    pub name: String,
    pub room_id: RoomId,
    pub components: Vec<ComponentSnapshot>,
    pub scripts: Vec<String>,
}

impl AgentPayloadSpec {
    /// Starts from an empty synthetic payload.
    pub fn synthetic(game_name: impl Into<String>) -> Self {
        Self::new(game_name.into(), AgentPayloadMode::Synthetic)
    }

    /// Starts from a seeded payload.
    pub fn seeded(game_name: impl Into<String>) -> Self {
        Self::new(game_name.into(), AgentPayloadMode::Seeded)
    }

    /// Adds a room to the current world.
    pub fn add_room(mut self, room_name: impl Into<String>) -> Self {
        self.current_world_mut().rooms.push(AgentRoomSpec {
            name: room_name.into(),
            entities: HashMap::new(),
        });
        self
    }

    /// Adds an entity to the first room of the current world.
    pub fn add_entity(mut self, entity_name: impl Into<String>) -> Self {
        let room = self.first_room_mut();
        room.entities
            .insert(entity_name.into(), AgentEntitySpec::default());
        self
    }

    /// Attaches a component by type name to a named entity.
    pub fn attach_component(mut self, entity_name: &str, type_name: &str, ron: &str) -> Self {
        let room = self.first_room_mut();
        let entity = room.entities.entry(entity_name.to_string()).or_default();
        entity.components.push(AgentComponentSpec {
            entity: entity_name.to_string(),
            type_name: type_name.to_string(),
            ron: ron.to_string(),
        });
        self
    }

    /// Attaches a script path to a named entity.
    pub fn attach_script(mut self, entity_name: &str, script_path: &str) -> Self {
        let room = self.first_room_mut();
        let entity = room.entities.entry(entity_name.to_string()).or_default();
        entity.scripts.push(script_path.to_string());
        self
    }

    /// Materializes the payload into an initialized game state.
    pub fn materialize(self, lua: &Lua) -> Result<AgentBuiltPayload, AgentPayloadError> {
        let mut built = self.build()?;
        built.game.initialize_runtime(lua);
        built.apply_entities(lua)?;
        Ok(built)
    }

    /// Builds a typed payload ready for runtime materialization.
    pub fn build(self) -> Result<AgentBuiltPayload, AgentPayloadError> {
        let world = self.worlds.first().ok_or(AgentPayloadError::MissingRoom)?;
        let room = world.rooms.first().ok_or(AgentPayloadError::MissingRoom)?;

        for entity in room.entities.values() {
            for component in &entity.components {
                if !component_type_is_registered(&component.type_name) {
                    return Err(AgentPayloadError::UnknownComponentType(
                        component.type_name.clone(),
                    ));
                }
            }
        }

        let mut game = Game::default();
        game.name = self.game_name.clone();

        let world_id = WorldId(Uuid::new_v4());
        let room_id = RoomId(1);
        let built_room = Room {
            id: room_id,
            name: room.name.clone(),
            ..Room::default()
        };
        let built_world = World {
            id: world_id,
            name: world.name.clone(),
            rooms: vec![built_room.clone()],
            current_room_id: Some(room_id),
            starting_room_id: Some(room_id),
            starting_position: None,
            meta: WorldMeta::default(),
            grid_size: 16.0,
        };

        game.worlds = vec![built_world];
        game.current_world_id = world_id;
        game.prefab_library = PrefabLibrary::default();

        let mut entity_specs: Vec<_> = room.entities.iter().collect();
        entity_specs.sort_by(|left, right| left.0.cmp(right.0));

        let entities = entity_specs
            .into_iter()
            .enumerate()
            .map(|(index, (name, entity))| AgentEntityExport {
                entity: Entity(index + 1),
                name: name.clone(),
                room_id,
                components: entity
                    .components
                    .iter()
                    .map(|component| ComponentSnapshot {
                        type_name: component.type_name.clone(),
                        ron: component.ron.clone(),
                    })
                    .collect(),
                scripts: entity.scripts.clone(),
            })
            .collect();

        Ok(AgentBuiltPayload {
            game,
            room: built_room,
            startup_mode: StartupMode::Skip,
            spec: self,
            entities,
        })
    }

    fn new(game_name: String, mode: AgentPayloadMode) -> Self {
        Self {
            game_name,
            mode,
            worlds: vec![AgentWorldSpec {
                name: "world".to_string(),
                rooms: Vec::new(),
            }],
        }
    }

    fn current_world_mut(&mut self) -> &mut AgentWorldSpec {
        self.worlds
            .first_mut()
            .expect("payload always starts with one world")
    }

    fn first_room_mut(&mut self) -> &mut AgentRoomSpec {
        if self.current_world_mut().rooms.is_empty() {
            self.current_world_mut().rooms.push(AgentRoomSpec {
                name: "room".to_string(),
                entities: HashMap::new(),
            });
        }

        &mut self.current_world_mut().rooms[0]
    }
}

impl AgentBuiltPayload {
    fn apply_entities(&mut self, lua: &Lua) -> Result<(), AgentPayloadError> {
        let room_id = self.room.id;
        let _ = lua;

        let entities = self.entities.clone();

        for export in entities {
            let entity = self.game.ecs.create_entity().finish();
            self.game
                .ecs
                .add_component_to_entity(entity, Name(export.name.clone()));
            self.game
                .ecs
                .add_component_to_entity(entity, CurrentRoom(room_id));

            for component in &export.components {
                self.apply_component_snapshot(entity, component)?;
            }

            for script_path in &export.scripts {
                let script_id = self
                    .game
                    .script_manager
                    .get_or_load(script_path)
                    .ok_or_else(|| AgentPayloadError::UnknownScriptType(script_path.clone()))?;
                self.game.ecs.add_component_to_entity(
                    entity,
                    Script {
                        script_id,
                        ..Default::default()
                    },
                );
            }
        }

        Ok(())
    }

    fn apply_component_snapshot(
        &mut self,
        entity: Entity,
        snapshot: &ComponentSnapshot,
    ) -> Result<(), AgentPayloadError> {
        if snapshot.type_name == comp_type_name::<Transform>() {
            let transform = ron::from_str::<Transform>(&snapshot.ron)
                .map_err(|_| AgentPayloadError::UnknownComponentType(snapshot.type_name.clone()))?;
            self.game.ecs.add_component_to_entity(entity, transform);
            return Ok(());
        }

        let component_reg = inventory::iter::<ComponentRegistry>()
            .find(|registry| registry.type_name == snapshot.type_name)
            .ok_or_else(|| AgentPayloadError::UnknownComponentType(snapshot.type_name.clone()))?;
        let mut boxed = (component_reg.from_ron_component)(snapshot.ron.clone());
        let mut ctx = self.game.ctx_mut();
        (component_reg.post_create)(&mut *boxed, &entity, &mut ctx);
        (component_reg.inserter)(ctx.ecs, entity, boxed);
        Ok(())
    }
}

fn component_type_is_registered(type_name: &str) -> bool {
    inventory::iter::<ComponentRegistry>()
        .into_iter()
        .any(|registry| registry.type_name == type_name)
}

#[cfg(test)]
mod tests;
