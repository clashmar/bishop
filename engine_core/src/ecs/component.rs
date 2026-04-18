use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;

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
    fn post_create(&mut self, ecs: &mut Ecs, entity: Entity, sprite_manager: &mut SpriteManager);
}

/// Returns the type name of a component.
#[inline]
pub fn comp_type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or_else(|| std::any::type_name::<T>())
}
