use crate::ecs::entity::Entity;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

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
