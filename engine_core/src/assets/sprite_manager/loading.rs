use super::*;
use crate::hydration::{EvictError, Hydratable};

impl SpriteManager {
    /// Load and initialize a texture from the assets folder.
    /// Returns the `SpriteId` for the texture.
    pub fn init_texture(
        &mut self,
        asset_registry: &mut AssetRegistry,
        loader: &impl TextureLoader,
        rel_path: impl AsRef<Path>,
    ) -> Result<SpriteId, String> {
        let path = rel_path.as_ref().to_path_buf();

        if path.to_string_lossy().trim().is_empty() {
            omni_info!("init_texture: empty path, returning error");
            return Err("Empty texture path".into());
        }

        // Path already registered - reuse the same id, but reload the texture if it was evicted.
        if let Some(&id) = self.path_to_sprite_id.get(&path) {
            if let std::collections::hash_map::Entry::Vacant(entry) = self.textures.entry(id) {
                self.pending_texture_reads.remove(&id);
                let texture = Self::load_texture_from_game(loader, &path)?;
                entry.insert(texture);
            }
            return Ok(id);
        }

        if self.next_sprite_id == 0 {
            self.restore_next_sprite_id();
        }

        let id = match asset_registry.key_for_path(assets_folder().join(&path)) {
            Some(AssetKey::Sprite(id)) => id,
            _ => SpriteId(self.next_sprite_id),
        };

        asset_registry
            .register_asset_relative_path(id, &path)
            .map_err(|error| error.to_string())?;

        self.path_to_sprite_id.insert(path.clone(), id);
        self.sprite_id_to_path.insert(id, path.clone());
        self.pending_texture_reads.remove(&id);

        self.restore_next_sprite_id();

        let texture = Self::load_texture_from_game(loader, &path)?;
        self.textures.insert(id, texture);

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
        self.pending_texture_reads.remove(id);
        Ok(())
    }

    /// Returns a texture from a `SpriteId`.
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
        if !self.sprite_id_to_path.contains_key(&id) {
            return self
                .empty_texture
                .get_or_insert_with(|| loader.empty_texture());
        }

        if self.runtime_texture_loading {
            self.prewarm_runtime_texture(id);
            self.poll_pending_texture_reads(loader);

            if self.textures.contains_key(&id) {
                return self.textures.get(&id).unwrap();
            }
        } else {
            let _ = self.ensure_loaded(loader, id);

            if self.textures.contains_key(&id) {
                return self.textures.get(&id).unwrap();
            }
        }

        self.empty_texture
            .get_or_insert_with(|| loader.empty_texture())
    }

    /// Returns the id for `path`, loading it if necessary.
    pub fn get_or_load<P: AsRef<Path>>(
        &mut self,
        asset_registry: &mut AssetRegistry,
        loader: &impl TextureLoader,
        path: P,
    ) -> Option<SpriteId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
        }

        match self.init_texture(asset_registry, loader, p) {
            Ok(id) => Some(id),
            Err(err) => {
                omni_error!("{}", err);
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

    /// Clears runtime texture state for a sprite id.
    pub fn evict_texture(&mut self, id: SpriteId) {
        self.decrement_ref(id);
        let _ = self.evict(&id);
    }

    /// Returns the registered relative path for a sprite id.
    pub fn path_for_id(&self, sprite_id: SpriteId) -> Option<&Path> {
        self.sprite_id_to_path.get(&sprite_id).map(PathBuf::as_path)
    }

    /// Returns the number of registered sprite ids.
    pub fn registered_id_count(&self) -> usize {
        self.sprite_id_to_path.len()
    }

    /// Ensures the texture for `id` is present in memory.
    pub fn ensure_loaded(
        &mut self,
        loader: &impl TextureLoader,
        id: SpriteId,
    ) -> Result<(), String> {
        if id.0 == 0 || self.textures.contains_key(&id) {
            return Ok(());
        }

        let Some(path) = self.sprite_id_to_path.get(&id).cloned() else {
            return Err(format!("Unknown sprite id: {:?}", id));
        };

        let texture = Self::load_texture_from_game(loader, &path)?;
        self.textures.insert(id, texture);
        self.path_to_sprite_id.insert(path, id);
        self.increment_ref(id);
        Ok(())
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
}

impl Hydratable for SpriteManager {
    type Id = SpriteId;

    /// Returns the reference count for a sprite.
    fn ref_count(&self, id: &Self::Id) -> usize {
        self.ref_counts.get(id).copied().unwrap_or(0)
    }

    /// Increments the reference count.
    fn increment_ref(&mut self, id: Self::Id) {
        *self.ref_counts.entry(id).or_insert(0) += 1;
    }

    /// Decrements the reference count.
    fn decrement_ref(&mut self, id: Self::Id) {
        debug_assert!(
            self.ref_counts.get(&id).copied().unwrap_or(0) > 0,
            "decrement_ref on SpriteId({}) with zero ref-count",
            id.0
        );
        if let Some(count) = self.ref_counts.get_mut(&id) {
            *count = count.saturating_sub(1);
        }
    }

    /// Attempts eviction. Fails if the ref-count is above zero.
    fn evict(&mut self, id: &Self::Id) -> Result<(), EvictError> {
        let count = self.ref_count(id);
        if count > 0 {
            return Err(EvictError::StillReferenced { count });
        }
        self.textures.remove(id);
        self.pending_texture_reads.remove(id);
        self.ref_counts.remove(id);
        Ok(())
    }
}
