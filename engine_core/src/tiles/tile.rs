// engine_core/src/tiles/tile.rs
use crate::assets::sprite::SpriteId;
use serde::{Deserialize, Serialize};

/// Identifier used by the editor and by the TileMap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TileDefId(pub usize);

/// A list of component adding closures.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TileDef {
    /// SpriteId for the tile.
    pub sprite_id: SpriteId,
    /// The list of tile components that the tile has.
    pub components: Vec<TileComponent>,
}

/// Serialisable description of a component.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum TileComponent {
    Walkable(bool),
    Solid(bool),
    Damage(f32),
}
