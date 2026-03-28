// engine_core/src/ecs/entity.rs
use crate::ecs::component::*;
use crate::ecs::component_registry::ComponentRegistry;
use crate::ecs::ecs::Ecs;
use ecs_component::ecs_component;
use inventory::iter;
use serde::{Deserialize, Serialize};
use std::any::TypeId;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize, Default)]
pub struct Entity(pub usize);

impl Entity {
    /// A sentinal value that can be used for optionals.
    pub fn null() -> Self {
        Entity(0)
    }
}

impl std::ops::Deref for Entity {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Entity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct EntityBuilder<'a> {
    pub id: Entity,
    pub ecs: &'a mut Ecs,
}

impl<'a> EntityBuilder<'a> {
    /// Attach any component that implements the `Component` marker trait.
    pub fn with<T>(self, comp: T) -> Self
    where
        T: Component + Default + 'static,
    {
        // Find the registration entry for `T`.
        let reg = iter::<ComponentRegistry>()
            .find(|r| r.type_id == TypeId::of::<ComponentStore<T>>())
            // TODO handle expect
            .expect("Component not registered.");

        // Insert `T` and every component listed in the macro’s requirement list.
        (reg.factory)(self.ecs, self.id);
        T::store_mut(self.ecs).insert(self.id, comp);
        self
    }

    /// Finish the builder and get the public `Entity` back.
    pub fn finish(self) -> Entity {
        self.id
    }
}

/// Parent entity reference for hierarchical relationships.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct Parent(pub Entity);

/// Children entities for hierarchical relationships.
#[ecs_component]
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Children {
    pub entities: Vec<Entity>,
}

impl Children {
    pub fn add(&mut self, child: Entity) {
        if !self.entities.contains(&child) {
            self.entities.push(child);
        }
    }

    pub fn remove(&mut self, child: Entity) {
        self.entities.retain(|&e| e != child);
    }

    pub fn contains(&self, child: Entity) -> bool {
        self.entities.contains(&child)
    }
}

/// Set the parent of an entity, updating both parent and child components.
pub fn set_parent(ecs: &mut Ecs, child: Entity, new_parent: Entity) {
    // Remove from old parent if it exists
    if let Some(old_parent) = ecs.get::<Parent>(child) {
        let old_parent_entity = old_parent.0;
        if let Some(children) = ecs.get_mut::<Children>(old_parent_entity) {
            children.remove(child);
        }
    }

    // Set new parent on child
    if let Some(parent_comp) = ecs.get_mut::<Parent>(child) {
        parent_comp.0 = new_parent;
    } else {
        ecs.get_store_mut::<Parent>()
            .insert(child, Parent(new_parent));
    }

    // Add child to new parent's children list
    if let Some(children) = ecs.get_mut::<Children>(new_parent) {
        children.add(child);
    } else {
        let mut children = Children::default();
        children.add(child);
        ecs.get_store_mut::<Children>().insert(new_parent, children);
    }
}

/// Remove parent relationship from a child.
pub fn remove_parent(ecs: &mut Ecs, child: Entity) {
    if let Some(parent) = ecs.get::<Parent>(child) {
        let parent_entity = parent.0;

        // Remove from parent's children list
        if let Some(children) = ecs.get_mut::<Children>(parent_entity) {
            children.remove(child);
        }

        // Remove parent component
        ecs.get_store_mut::<Parent>().remove(child);
    }
}

/// Get all children of an entity.
pub fn get_children(ecs: &Ecs, entity: Entity) -> Vec<Entity> {
    ecs.get::<Children>(entity)
        .map(|c| c.entities.clone())
        .unwrap_or_default()
}

/// Get the parent of an entity.
pub fn get_parent(ecs: &Ecs, entity: Entity) -> Option<Entity> {
    ecs.get::<Parent>(entity).map(|p| p.0)
}

/// Check if an entity has any children.
pub fn has_children(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get::<Children>(entity)
        .map(|c| !c.entities.is_empty())
        .unwrap_or(false)
}

/// Get all root entities (entities without parents) in a given set.
pub fn get_root_entities(ecs: &Ecs, entities: &[Entity]) -> Vec<Entity> {
    entities
        .iter()
        .copied()
        .filter(|&e| !ecs.has::<Parent>(e))
        .collect()
}

/// Recursively get all descendants of an entity.
pub fn get_all_descendants(ecs: &Ecs, entity: Entity) -> Vec<Entity> {
    let mut descendants = Vec::new();
    let children = get_children(ecs, entity);

    for child in children {
        descendants.push(child);
        descendants.extend(get_all_descendants(ecs, child));
    }

    descendants
}

/// Check if an entity is an ancestor of another entity.
pub fn is_ancestor(ecs: &Ecs, potential_ancestor: Entity, entity: Entity) -> bool {
    let mut current = entity;
    while let Some(parent) = get_parent(ecs, current) {
        if parent == potential_ancestor {
            return true;
        }
        current = parent;
    }
    false
}
