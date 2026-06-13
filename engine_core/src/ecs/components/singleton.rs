use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marks an entity as an engine-managed singleton, hidden from the editor entity list.
#[ecs_component(lua_api = false)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct Singleton;
