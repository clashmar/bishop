// engine_core/src/assets/asset_manager.rs
use crate::animation::animation_clip::Animation;
use crate::assets::sprite::*;
use crate::game::Game;
use crate::storage::path_utils::assets_folder;
use crate::tiles::tile::*;
use crate::*;
use bishop::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct AssetManager {
    #[serde(skip)]
    textures: HashMap<SpriteId, Texture2D>,
    /// Persistent map of all sprite ids to their paths.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
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
    /// Reference counts for sprite ids.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    ref_counts: HashMap<SpriteId, usize>,
    /// Sprite ids whose path mappings should be removed on exit.
    #[cfg(feature = "editor")]
    #[serde(skip)]
    pending_path_removal: HashSet<SpriteId>,
    /// Placeholder texture returned for unset or missing sprite ids.
    #[serde(skip)]
    empty_texture: Option<Texture2D>,
}

impl AssetManager {
    /// Load and initialize a texture from the assets folder.
    /// Returns the `SpriteId` for the texture.
    pub fn init_texture(
        &mut self,
        loader: &impl TextureLoader,
        rel_path: impl AsRef<Path>,
    ) -> Result<SpriteId, String> {
        let path = rel_path.as_ref().to_path_buf();

        if path.to_string_lossy().trim().is_empty() {
            onscreen_info!("init_texture: empty path, returning error");
            return Err("Empty texture path".into());
        }

        // Path already registered — reuse the same id, but reload the texture if it was evicted.
        if let Some(&id) = self.path_to_sprite_id.get(&path) {
            if let std::collections::hash_map::Entry::Vacant(entry) = self.textures.entry(id) {
                let texture = Self::load_texture_from_game(loader, &path)?;
                entry.insert(texture);
            }
            return Ok(id);
        }

        // Load the texture from the assets folder.
        let texture = Self::load_texture_from_game(loader, &path)?;

        // Assign the next texture id
        let id = SpriteId(self.next_sprite_id);

        // Store everything
        self.textures.insert(id, texture);
        self.path_to_sprite_id.insert(path.clone(), id);
        self.sprite_id_to_path.insert(id, path.clone());

        // Calculate next available id AFTER inserting
        self.restore_next_sprite_id();

        info!(
            "init_texture: loaded {:?} as {:?}, next_sprite_id now {}",
            path, id, self.next_sprite_id
        );

        Ok(id)
    }

    /// Reloads a texture from its `SpriteId` and updates `path_to_sprite_id`.
    pub fn reload_texture(
        &mut self,
        loader: &impl TextureLoader,
        id: &SpriteId,
        path: &Path,
    ) -> Result<(), String> {
        let texture = Self::load_texture_from_game(loader, path)?;
        self.textures.insert(*id, texture);
        self.path_to_sprite_id.insert(path.to_path_buf(), *id);
        Ok(())
    }

    /// Returns a texture from a `SpriteId`. Loads lazily if evicted.
    pub fn get_texture_from_id(&mut self, loader: &impl TextureLoader, id: SpriteId) -> &Texture2D {
        if id.0 == 0 {
            return self
                .empty_texture
                .get_or_insert_with(|| loader.empty_texture());
        }

        if self.textures.contains_key(&id) {
            return self.textures.get(&id).unwrap();
        }

        // Look up the original path and load lazily.
        let path = match self.sprite_id_to_path.get(&id) {
            Some(p) => p.clone(),
            None => {
                return self
                    .empty_texture
                    .get_or_insert_with(|| loader.empty_texture());
            }
        };

        let _ = self.init_texture(loader, &path);

        if self.textures.contains_key(&id) {
            self.textures.get(&id).unwrap()
        } else {
            self.empty_texture
                .get_or_insert_with(|| loader.empty_texture())
        }
    }

    /// Returns the id for `path`, loading it if necessary.
    pub fn get_or_load<P: AsRef<Path>>(
        &mut self,
        loader: &impl TextureLoader,
        path: P,
    ) -> Option<SpriteId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
        }

        if let Some(&id) = self.path_to_sprite_id.get(p) {
            return Some(id);
        }

        match self.init_texture(loader, p) {
            Ok(id) => Some(id),
            Err(err) => {
                onscreen_error!("{}", err);
                None
            }
        }
    }

    /// Returns the id for `path` or `None` if not loaded.
    pub fn get_or_none<P: AsRef<Path>>(&self, path: P) -> Option<SpriteId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
        }
        if let Some(&id) = self.path_to_sprite_id.get(p) {
            return Some(id);
        }
        None
    }

    /// Initialize all assets for the game.
    pub fn init_manager(loader: &impl TextureLoader, game: &mut Game) {
        // Calculate the next id from the existing map
        game.asset_manager.restore_next_sprite_id();

        // Restore next tile def id
        if let Some(max_id) = game.asset_manager.tile_defs.keys().map(|id| id.0).max() {
            game.asset_manager.next_tile_def_id = max_id + 1;
        } else {
            game.asset_manager.next_tile_def_id = 1;
        }

        let sprites: Vec<(SpriteId, PathBuf)> = game
            .asset_manager
            .sprite_id_to_path
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect();

        // Reload all textures first
        for (id, path) in sprites {
            let _ = game.asset_manager.reload_texture(loader, &id, &path);
        }

        for animation in game.ecs.get_store_mut::<Animation>().data.values_mut() {
            animation.init_sprite_cache(loader, &mut game.asset_manager);
            animation.init_runtime();
        }
    }

    /// Returns a path normalized relative to the game's assets folder.
    pub fn normalize_path(&self, path: PathBuf) -> PathBuf {
        let assets_dir = assets_folder();
        path.strip_prefix(&assets_dir)
            .unwrap_or_else(|_| &path)
            .to_path_buf()
    }

    /// Returns true if the texture for `id` is already present.
    #[inline]
    pub fn contains(&self, id: SpriteId) -> bool {
        self.textures.contains_key(&id)
    }

    /// Return the pixel width and height of the texture that belongs to `id`
    /// or None if the texture has not been loaded/set.
    pub fn texture_size(&self, id: SpriteId) -> Option<(f32, f32)> {
        self.textures
            .get(&id)
            .map(|tex| (tex.width(), tex.height()))
    }

    /// Returns the number of loaded textures.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Returns the number of tile definitions.
    pub fn tile_def_count(&self) -> usize {
        self.tile_defs.len()
    }

    /// Increment reference count for a sprite.
    pub fn increment_ref(&mut self, sprite_id: SpriteId) {
        if sprite_id.0 == 0 {
            return;
        }

        *self.ref_counts.entry(sprite_id).or_insert(0) += 1;

        #[cfg(feature = "editor")]
        {
            self.pending_path_removal.remove(&sprite_id);
        }
    }

    /// Decrement reference count for a sprite, cleaning up all structures when count reaches zero.
    pub fn decrement_ref(&mut self, sprite_id: SpriteId) {
        if sprite_id.0 == 0 {
            return;
        }

        if let Some(count) = self.ref_counts.get_mut(&sprite_id) {
            *count = count.saturating_sub(1);

            if *count == 0 {
                self.ref_counts.remove(&sprite_id);
                self.textures.remove(&sprite_id);

                #[cfg(feature = "editor")]
                {
                    self.pending_path_removal.insert(sprite_id);
                }
            }
        }
    }

    /// Remove path mappings for all sprites with a zero ref count.
    /// Call this before serializing game data on exit.
    #[cfg(feature = "editor")]
    pub fn flush_pending_removals(&mut self) {
        for id in self.pending_path_removal.drain() {
            if let Some(path) = self.sprite_id_to_path.remove(&id) {
                self.path_to_sprite_id.remove(&path);
            }
        }
    }

    /// Returns the reference count for a sprite.
    pub fn get_ref_count(&self, sprite_id: SpriteId) -> usize {
        self.ref_counts.get(&sprite_id).copied().unwrap_or(0)
    }

    /// Changes a sprite reference, handling decrement of old and increment of new.
    pub fn change_sprite(&mut self, old_id: &mut SpriteId, new_id: SpriteId) {
        if *old_id == new_id {
            return;
        }

        self.decrement_ref(*old_id);
        *old_id = new_id;
        self.increment_ref(new_id);
    }

    /// Changes an optional sprite reference, handling decrement of old and increment of new.
    pub fn change_sprite_option(
        &mut self,
        old_id: &mut Option<SpriteId>,
        new_id: Option<SpriteId>,
    ) {
        if *old_id == new_id {
            return;
        }

        if let Some(old) = *old_id {
            self.decrement_ref(old);
        }

        if let Some(new) = new_id {
            self.increment_ref(new);
        }

        *old_id = new_id;
    }

    /// Loads a texture from the assets folder using the provided loader.
    fn load_texture_from_game<P: AsRef<Path>>(
        loader: &impl TextureLoader,
        rel_path: P,
    ) -> Result<Texture2D, String> {
        let full_path = assets_folder().join(rel_path.as_ref());
        loader
            .load_texture_from_path(full_path.to_string_lossy().as_ref())
            .map_err(|e| {
                format!(
                    "Failed to load texture '{}': {}",
                    rel_path.as_ref().display(),
                    e
                )
            })
    }

    /// Calculates the next sprite id.
    pub fn restore_next_sprite_id(&mut self) {
        let used: HashSet<_> = self
            .sprite_id_to_path
            .keys()
            .filter_map(|sid| {
                let id = sid.0;
                if id == 0 {
                    // Skip sentinel value 0
                    None
                } else {
                    Some(id)
                }
            })
            .collect();

        let mut candidate = 1usize;

        // Scan through until an unused id is found
        while used.contains(&candidate) {
            candidate += 1;
        }

        self.next_sprite_id = candidate;
    }

    /// Inserts a TileDef and returns its id, incrementing sprite ref count.
    pub fn insert_tile_def(&mut self, def: TileDef) -> TileDefId {
        let id = TileDefId(self.next_tile_def_id);
        self.next_tile_def_id += 1;
        self.increment_ref(def.sprite_id);
        self.tile_defs.insert(id, def);
        id
    }

    /// Deletes a TileDef by id, decrementing sprite ref count.
    pub fn delete_tile_def(&mut self, id: TileDefId) {
        if let Some(def) = self.tile_defs.remove(&id) {
            self.decrement_ref(def.sprite_id);
        }
    }

    /// Updates a TileDef's sprite, handling ref counting for the change.
    pub fn update_tile_def_sprite(&mut self, id: TileDefId, new_sprite_id: SpriteId) {
        let old_sprite_id = self.tile_defs.get(&id).map(|def| def.sprite_id);

        if let Some(old_id) = old_sprite_id
            && old_id != new_sprite_id
        {
            self.decrement_ref(old_id);
            self.increment_ref(new_sprite_id);
            if let Some(def) = self.tile_defs.get_mut(&id) {
                def.sprite_id = new_sprite_id;
            }
        }
    }
}
