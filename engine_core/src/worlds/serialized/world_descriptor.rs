use crate::ecs::Entity;
#[cfg(test)]
use crate::constants::world;
use crate::scripting::event_tags::event_tag::EventTag;
use crate::worlds::room::{Exit, Room, RoomId};
use crate::worlds::world::{default_world_gravity, World, WorldId, WorldMeta};
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::FromInto;

/// A single room entry inside a world descriptor.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoomDirectoryEntry {
    pub id: RoomId,
    pub name: String,
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub position: Vec2,
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub size: Vec2,
    pub exits: Vec<Exit>,
    pub adjacent_rooms: Vec<RoomId>,
    pub tags: Vec<EventTag>,
    pub singleton: Entity,
}

/// Lightweight world descriptor loaded at boot for topology and traversal planning.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldDescriptor {
    pub id: WorldId,
    pub name: String,
    pub current_room_id: Option<RoomId>,
    pub meta: WorldMeta,
    pub tags: Vec<EventTag>,
    pub overlay: bool,
    pub grid_size: f32,
    #[serde(default = "default_world_gravity")]
    pub gravity: f32,
    pub singleton: Entity,
    pub rooms: Vec<RoomDirectoryEntry>,
}

impl World {
    /// Builds a lightweight `World` shell from a descriptor.
    /// Room variants are left empty; they are filled when the room payload hydrates.
    pub fn from_descriptor(descriptor: WorldDescriptor) -> Self {
        let mut world = World::new(descriptor.id, descriptor.name, descriptor.grid_size);
        world.current_room_id = descriptor.current_room_id;
        world.meta = descriptor.meta;
        world.tags = descriptor.tags;
        world.overlay = descriptor.overlay;
        world.gravity = descriptor.gravity;
        world.singleton = descriptor.singleton;
        for entry in descriptor.rooms {
            let room = Room {
                id: entry.id,
                name: entry.name,
                position: entry.position,
                size: entry.size,
                exits: entry.exits,
                adjacent_rooms: entry.adjacent_rooms,
                tags: entry.tags,
                variants: vec![],
                darkness: 0.0,
                singleton: entry.singleton,
            };
            world.add_room(room);
        }
        world
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_descriptor_builds_room_shells_without_room_payloads() {
        let descriptor = WorldDescriptor {
            id: WorldId(1),
            name: "Overworld".to_string(),
            current_room_id: Some(RoomId(7)),
            meta: WorldMeta::default(),
            tags: vec![],
            overlay: false,
            grid_size: 16.0,
            gravity: world::DEFAULT_WORLD_GRAVITY,
            singleton: Entity(9),
            rooms: vec![RoomDirectoryEntry {
                id: RoomId(7),
                name: "Spawn".to_string(),
                position: Vec2::ZERO,
                size: Vec2::new(16.0, 9.0),
                exits: vec![],
                adjacent_rooms: vec![],
                tags: vec![],
                singleton: Entity(7),
            }],
        };

        let world = World::from_descriptor(descriptor.clone());
        let room = world.get_room(RoomId(7)).unwrap();

        assert_eq!(world.id, descriptor.id);
        assert_eq!(world.singleton, descriptor.singleton);
        assert_eq!(room.singleton, Entity(7));
        assert!(room.variants.is_empty());
    }

    #[test]
    fn world_descriptor_deserialization_without_gravity_uses_default() {
        let descriptor: WorldDescriptor = ron::from_str(
            r#"(
                id: (1),
                name: "Overworld",
                current_room_id: Some((7)),
                meta: (position: (0.0, 0.0), sprite_id: None),
                tags: [],
                overlay: false,
                grid_size: 16.0,
                singleton: (9),
                rooms: [],
            )"#,
        )
        .unwrap();

        assert_eq!(descriptor.gravity, world::DEFAULT_WORLD_GRAVITY);
    }

    #[test]
    fn world_from_descriptor_copies_gravity() {
        let descriptor = WorldDescriptor {
            id: WorldId(1),
            name: "Overworld".to_string(),
            current_room_id: Some(RoomId(7)),
            meta: WorldMeta::default(),
            tags: vec![],
            overlay: false,
            grid_size: 16.0,
            gravity: 25.0,
            singleton: Entity(9),
            rooms: vec![],
        };

        let world = World::from_descriptor(descriptor);

        assert_eq!(world.gravity, 25.0);
    }
}
