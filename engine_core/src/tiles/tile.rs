use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::{Component, comp_type_name};
use crate::ecs::component_registry::COMPONENTS;
use crate::ecs::SpriteId;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Identifier used by the editor and by the TileMap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TileDefId(pub usize);

/// Authored tile definition.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct TileDef {
    /// SpriteId for the tile.
    pub sprite_id: SpriteId,
    /// Authored ECS component snapshots owned by this tile definition.
    #[serde(default)]
    pub components: Vec<ComponentSnapshot>,
}

/// Builds one serialized component snapshot for tile-definition authoring/tests.
pub fn tile_definition_component_snapshot<T>(component: T) -> ComponentSnapshot
where
    T: Component + Any + 'static,
{
    let type_name = comp_type_name::<T>();
    let reg = COMPONENTS
        .iter()
        .find(|reg| reg.type_name == type_name)
        .unwrap_or_else(|| panic!("missing registry entry for {type_name}"));
    let ron = (reg.to_ron_component)(&component as &dyn Any);

    ComponentSnapshot {
        type_name: reg.type_name.to_string(),
        ron,
    }
}
