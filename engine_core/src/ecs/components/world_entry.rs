use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marks an entity as a named world entry point; arrivals use its `CurrentRoom` and `Transform`.
#[ecs_component]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct WorldEntry {
    /// Entry point name, unique within its world.
    pub name: String,
}
inspector_module!(WorldEntry);
