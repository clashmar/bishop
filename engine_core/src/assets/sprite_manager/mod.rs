mod loading;
mod refs;
mod registry;
mod runtime;

use crate::assets::asset_manager::{AssetManager, IdPathAssetManager};
use crate::assets::asset_registry::AssetKey;
use crate::assets::AssetRegistry;
use crate::ecs::{Animation, SpriteId};
use crate::game::Game;
use crate::storage::path_utils::assets_folder;
use crate::task::FileReadPool;
use crate::tiles::tile::*;
use crate::*;
use bishop::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

const SPRITE_ASSET_KIND: &str = "Sprite";

#[derive(Serialize, Deserialize, Default)]
pub struct SpriteManager {
    #[serde(skip)]
    textures: HashMap<SpriteId, Texture2D>,
    /// Derived cache of all sprite ids to their paths.
    #[serde(skip)]
    pub sprite_id_to_path: HashMap<SpriteId, PathBuf>,
    #[serde(skip)]
    pub path_to_sprite_id: HashMap<PathBuf, SpriteId>,
    #[serde(skip)]
    /// Counter for sprite ids. Starts from 1.
    next_sprite_id: usize,
    /// Maps `TileDefIds` to `TileDef`.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    pub tile_defs: HashMap<TileDefId, TileDef>,
    /// Counter for tile def ids. Starts from 1.
    next_tile_def_id: usize,
    #[serde(skip)]
    runtime_texture_loading: bool,
    #[serde(skip)]
    runtime_file_read_pool: Option<FileReadPool>,
    #[serde(skip)]
    pending_texture_reads: HashMap<SpriteId, PathBuf>,
    /// Placeholder texture returned for unset or missing sprite ids.
    #[serde(skip)]
    empty_texture: Option<Texture2D>,
}

impl AssetManager for SpriteManager {
    fn editor_metadata_snapshot(&self) -> Self {
        Self {
            tile_defs: self.tile_defs.clone(),
            ..Default::default()
        }
    }

    fn merge_editor_metadata_from(&mut self, source: &Self) -> std::io::Result<()> {
        self.tile_defs = source.tile_defs.clone();
        Ok(())
    }
}

impl IdPathAssetManager for SpriteManager {
    type AssetId = SpriteId;

    fn asset_kind() -> &'static str {
        SPRITE_ASSET_KIND
    }

    fn id_to_path(&self) -> &HashMap<Self::AssetId, PathBuf> {
        &self.sprite_id_to_path
    }

    fn id_to_path_mut(&mut self) -> &mut HashMap<Self::AssetId, PathBuf> {
        &mut self.sprite_id_to_path
    }

    fn path_to_id(&self) -> &HashMap<PathBuf, Self::AssetId> {
        &self.path_to_sprite_id
    }

    fn path_to_id_mut(&mut self) -> &mut HashMap<PathBuf, Self::AssetId> {
        &mut self.path_to_sprite_id
    }

    fn rebuild_editor_metadata(&mut self) {
        self.restore_next_sprite_id();
    }
}

#[cfg(test)]
mod tests;
