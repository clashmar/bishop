// engine_core/src/ecs/component.rs
use crate::assets::asset_manager::AssetManager;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::inspector_module;
use crate::worlds::room::RoomId;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;

/// Marker trait for components.
pub trait Component: Send + Sync {
    fn store_mut(world: &mut Ecs) -> &mut ComponentStore<Self>
    where
        Self: Sized;

    fn store(world: &Ecs) -> &ComponentStore<Self>
    where
        Self: Sized;
}

pub struct ComponentStore<T> {
    pub data: HashMap<Entity, T>,
}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        ComponentStore {
            data: HashMap::new(),
        }
    }
}

impl<T> Serialize for ComponentStore<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::storage::ordered_map::serialize(&self.data, serializer)
    }
}

impl<'de, T> Deserialize<'de> for ComponentStore<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = crate::storage::ordered_map::deserialize(deserializer)?;
        Ok(Self { data })
    }
}

impl<T> ComponentStore<T> {
    pub fn insert(&mut self, entity: Entity, component: T) {
        self.data.insert(entity, component);
    }
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(&entity)
    }
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(&entity)
    }
    pub fn remove(&mut self, entity: Entity) {
        self.data.remove(&entity);
    }
    pub fn contains(&self, entity: Entity) -> bool {
        self.data.contains_key(&entity)
    }
}

/// Component bag that can remembers components for a entity and can restore them.
pub struct ComponentEntry {
    /// The concrete component value.
    pub value: Box<dyn Any>,
    /// Function that can clone the boxed value.
    pub cloner: fn(&dyn Any) -> Box<dyn Any>,
}

impl Clone for ComponentEntry {
    fn clone(&self) -> Self {
        Self {
            value: (self.cloner)(&*self.value),
            cloner: self.cloner,
        }
    }
}

/// Can be alled once a component has been added to an entity to initialize it.
pub trait PostCreate {
    fn post_create(&mut self, ecs: &mut Ecs, entity: Entity, asset_manager: &mut AssetManager);
}

/// Returns the type name of a component.
#[inline]
pub fn comp_type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or_else(|| std::any::type_name::<T>())
}

/// The human readable name of the entity.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default, Reflect)]
pub struct Name(pub String);
inspector_module!(Name, removable = false);

impl Deref for Name {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Marker trait for global components.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Global {}

/// Z layer of an entity.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Reflect)]
#[serde(default)]
pub struct Layer {
    pub z: i32,
}
inspector_module!(Layer);

/// Component that stores the room identifier an entity belongs to.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CurrentRoom(pub RoomId);

/// Marker component for the player entity.
#[ecs_component(deps = [Collider, Velocity])]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Player;

/// Marker component for player proxies in rooms.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PlayerProxy;

#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Reflect)]
#[serde(default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[ecs_component]
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Grounded(#[serde(skip)] pub bool);

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
}
inspector_module!(Collider);

impl Default for Collider {
    fn default() -> Self {
        Self {
            width: 16.0,
            height: 16.0,
        }
    }
}

/// Accumulated sub-pixel remainder for pixel-perfect physics.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct SubPixel {
    #[serde(skip)]
    pub x: f32,
    #[serde(skip)]
    pub y: f32,
}

/// Marker for participation in the physics system.
#[ecs_component(deps = [Grounded, SubPixel])]
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicsBody;

/// Marker for entities that move by code.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct Kinematic {}

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct Walkable(pub bool);

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default, Reflect)]
pub struct Solid(pub bool);
inspector_module!(Solid);

#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default, Reflect)]
pub struct Damage {
    pub amount: f32,
}
