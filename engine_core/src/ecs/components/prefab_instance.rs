use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::entity::Entity;
use crate::prefab::PrefabId;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Marks the root entity for a linked prefab instance.
#[ecs_component(lua_api = false)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrefabInstanceRoot {
    /// Stable prefab asset id for this linked instance.
    pub prefab_id: PrefabId,
}

/// Marks an entity as belonging to a linked prefab node.
#[ecs_component(lua_api = false)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PrefabInstanceNode {
    /// Stable prefab asset id for this linked instance.
    pub prefab_id: PrefabId,
    /// Stable prefab node id within the asset.
    pub node_id: usize,
    /// Root entity for the linked instance subtree.
    pub root_entity: Entity,
}

/// Stores local divergence from the source prefab definition.
#[ecs_component(lua_api = false)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrefabOverrides {
    /// Component type names modified locally on this instance entity.
    pub modified_components: Vec<String>,
    /// Component type names removed locally on this instance entity.
    pub removed_components: Vec<String>,
    /// Components added locally on this instance entity.
    pub added_components: Vec<ComponentSnapshot>,
}
