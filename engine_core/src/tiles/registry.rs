use crate::tiles::{TileDef, TileDefId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stores authored tile definitions by id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TileRegistry {
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    definitions: HashMap<TileDefId, TileDef>,
    next_tile_def_id: usize,
}

impl TileRegistry {
    /// Inserts a tile definition and returns its id.
    pub fn insert(&mut self, def: TileDef) -> TileDefId {
        let id = TileDefId(self.next_tile_def_id.max(1));
        self.next_tile_def_id = id.0 + 1;
        self.definitions.insert(id, def);
        id
    }

    /// Returns the tile definition for `id`.
    pub fn get(&self, id: TileDefId) -> Option<&TileDef> {
        self.definitions.get(&id)
    }

    /// Replaces the definition stored at `id`.
    pub fn replace(&mut self, id: TileDefId, def: TileDef) {
        self.definitions.insert(id, def);
        self.next_tile_def_id = self.next_tile_def_id.max(id.0 + 1);
    }

    /// Returns an iterator over stored tile definitions.
    pub fn iter(&self) -> impl Iterator<Item = (TileDefId, &TileDef)> {
        self.definitions.iter().map(|(&id, def)| (id, def))
    }

    /// Removes the tile definition stored at `id`.
    pub fn remove(&mut self, id: TileDefId) -> Option<TileDef> {
        self.definitions.remove(&id)
    }

    /// Returns the number of stored tile definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::SpriteId;
    use crate::tiles::TileComponent;

    #[test]
    fn tile_registry_round_trip_when_serialized_then_tile_definitions_persist() {
        let mut registry = TileRegistry::default();
        let tile_id = registry.insert(TileDef {
            sprite_id: SpriteId(7),
            components: vec![TileComponent::Solid(true)],
        });

        let ron = ron::to_string(&registry).expect("tile registry should serialize");
        let loaded: TileRegistry = ron::from_str(&ron).expect("tile registry should deserialize");

        let def = loaded.get(tile_id).expect("tile definition should survive round-trip");
        assert_eq!(def.sprite_id, SpriteId(7));
        assert_eq!(def.components, vec![TileComponent::Solid(true)]);
    }
}
