// engine_core/src/assets/asset_manager.rs
use crate::animation::animation_clip::Animation;
use crate::assets::sprite::*;
use crate::ecs::ecs::Ecs;
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
                self.pending_texture_reads.remove(&id);
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
        self.pending_texture_reads.remove(&id);

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
            self.queue_runtime_texture_read(id);
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
        loader: &impl TextureLoader,
        path: P,
    ) -> Option<SpriteId> {
        let p = path.as_ref();
        if p.to_string_lossy().trim().is_empty() {
            return None;
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
        Self::init_editor_services(loader, &mut game.ecs, &mut game.asset_manager);
    }

    /// Initialize editor asset metadata without hydrating textures.
    pub fn init_editor_metadata(asset_manager: &mut AssetManager) {
        asset_manager.restore_next_sprite_id();
        asset_manager.runtime_texture_loading = false;
        asset_manager.runtime_file_read_pool = None;
        asset_manager.pending_texture_reads.clear();

        asset_manager.path_to_sprite_id.clear();
        for (&id, path) in &asset_manager.sprite_id_to_path {
            asset_manager.path_to_sprite_id.insert(path.clone(), id);
        }

        if let Some(max_id) = asset_manager.tile_defs.keys().map(|id| id.0).max() {
            asset_manager.next_tile_def_id = max_id + 1;
        } else {
            asset_manager.next_tile_def_id = 1;
        }
    }

    /// Initialize editor asset services for an ECS/asset-manager pair.
    pub fn init_editor_services(
        loader: &impl TextureLoader,
        ecs: &mut Ecs,
        asset_manager: &mut AssetManager,
    ) {
        Self::init_editor_metadata(asset_manager);

        let sprites: Vec<(SpriteId, PathBuf)> = asset_manager
            .sprite_id_to_path
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect();

        for (id, path) in sprites {
            let _ = asset_manager.reload_texture(loader, &id, &path);
        }

        for animation in ecs.get_store_mut::<Animation>().data.values_mut() {
            animation.init_sprite_cache(loader, asset_manager);
            animation.init_runtime();
        }
    }

    /// Initialize game data for runtime startup without hydrating all textures up front.
    pub fn init_runtime_manager(game: &mut Game) {
        let file_read_pool = FileReadPool::new();
        Self::init_runtime_manager_with_pool(&file_read_pool, game);
    }

    /// Initialize runtime data with an explicit file-read pool handle.
    pub fn init_runtime_manager_with_pool(file_read_pool: &FileReadPool, game: &mut Game) {
        game.asset_manager.restore_next_sprite_id();
        game.asset_manager.runtime_texture_loading = true;
        game.asset_manager.runtime_file_read_pool = Some(file_read_pool.clone());
        game.asset_manager.pending_texture_reads.clear();

        if let Some(max_id) = game.asset_manager.tile_defs.keys().map(|id| id.0).max() {
            game.asset_manager.next_tile_def_id = max_id + 1;
        } else {
            game.asset_manager.next_tile_def_id = 1;
        }

        for animation in game.ecs.get_store_mut::<Animation>().data.values_mut() {
            animation.init_sprite_cache_runtime(&game.asset_manager);
            animation.init_runtime();
        }
    }

    fn queue_runtime_texture_read(&mut self, id: SpriteId) {
        if !self.runtime_texture_loading || id.0 == 0 {
            return;
        }

        let Some(file_read_pool) = self.runtime_file_read_pool.as_ref() else {
            return;
        };

        if self.textures.contains_key(&id) || self.pending_texture_reads.contains_key(&id) {
            return;
        }

        let Some(path) = self.sprite_id_to_path.get(&id).cloned() else {
            return;
        };

        let full_path = assets_folder().join(&path);
        file_read_pool.queue_read(id.0.to_string(), full_path);
        self.pending_texture_reads.insert(id, path);
    }

    fn poll_pending_texture_reads(&mut self, loader: &impl TextureLoader) {
        let Some(file_read_pool) = self.runtime_file_read_pool.clone() else {
            return;
        };

        while let Some(completed) = file_read_pool.try_recv_completed() {
            let Ok(sprite_index) = completed.id.parse::<usize>() else {
                continue;
            };
            let sprite_id = SpriteId(sprite_index);

            let Some(path) = self.pending_texture_reads.get(&sprite_id).cloned() else {
                continue;
            };

            if completed.path != assets_folder().join(&path) {
                continue;
            }

            self.pending_texture_reads.remove(&sprite_id);
            let path_display = path.display().to_string();

            match completed.result {
                Ok(bytes) => match loader.load_texture_from_bytes(&bytes) {
                    Ok(texture) => {
                        self.textures.insert(sprite_id, texture);
                        self.path_to_sprite_id.insert(path, sprite_id);
                    }
                    Err(error) => {
                        onscreen_error!("Failed to upload texture '{}': {}", path_display, error);
                    }
                },
                Err(error) => {
                    onscreen_error!("Failed to read texture '{}': {}", path_display, error);
                }
            }
        }
    }

    #[cfg(test)]
    fn enable_runtime_texture_loading_for_test(&mut self) {
        self.runtime_texture_loading = true;
    }

    #[cfg(test)]
    fn attach_runtime_file_read_pool_for_test(&mut self, file_read_pool: &FileReadPool) {
        self.runtime_file_read_pool = Some(file_read_pool.clone());
    }

    #[cfg(test)]
    fn has_pending_texture_read(&self, id: SpriteId) -> bool {
        self.pending_texture_reads.contains_key(&id)
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct CountingFailingLoader {
        bytes_load_calls: Cell<usize>,
        load_calls: Cell<usize>,
    }

    impl CountingFailingLoader {
        fn new() -> Self {
            Self {
                bytes_load_calls: Cell::new(0),
                load_calls: Cell::new(0),
            }
        }
    }

    impl TextureLoader for CountingFailingLoader {
        fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
            self.bytes_load_calls
                .set(self.bytes_load_calls.get().saturating_add(1));
            Err("expected test byte load failure".to_string())
        }

        fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
            self.load_calls.set(self.load_calls.get() + 1);
            Err("expected test load failure".to_string())
        }

        fn empty_texture(&self) -> Texture2D {
            panic!("empty_texture is not used in asset manager tests")
        }
    }

    #[test]
    fn get_or_load_retries_loader_for_registered_path_with_missing_texture() {
        let loader = CountingFailingLoader::new();
        let mut asset_manager = AssetManager::default();
        let path = PathBuf::from("sprites/player.png");
        let sprite_id = SpriteId(7);

        asset_manager
            .path_to_sprite_id
            .insert(path.clone(), sprite_id);
        asset_manager
            .sprite_id_to_path
            .insert(sprite_id, path.clone());

        let result = asset_manager.get_or_load(&loader, &path);

        assert!(result.is_none());
        assert_eq!(loader.load_calls.get(), 1);
    }

    #[test]
    fn ensure_loaded_retries_loader_for_registered_sprite_id_with_missing_texture() {
        let loader = CountingFailingLoader::new();
        let mut asset_manager = AssetManager::default();
        let path = PathBuf::from("sprites/player.png");
        let sprite_id = SpriteId(7);

        asset_manager
            .path_to_sprite_id
            .insert(path.clone(), sprite_id);
        asset_manager
            .sprite_id_to_path
            .insert(sprite_id, path.clone());

        let result = asset_manager.ensure_loaded(&loader, sprite_id);

        assert!(result.is_err());
        assert_eq!(loader.load_calls.get(), 1);
    }

    #[test]
    fn queue_runtime_texture_read_tracks_pending_sprite_id() {
        let mut asset_manager = AssetManager::default();
        let file_read_pool = FileReadPool::new();
        let path = PathBuf::from(format!(
            "textures/runtime-queue-{}.bin",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX_EPOCH")
                .as_nanos()
        ));
        let sprite_id = SpriteId(7);
        let full_path = assets_folder().join(&path);

        fs::create_dir_all(
            full_path
                .parent()
                .expect("runtime queue test path should have a parent"),
        )
        .expect("runtime queue test directory should be writable");
        fs::write(&full_path, [1_u8, 2, 3, 4]).expect("runtime queue test file should be writable");

        asset_manager
            .path_to_sprite_id
            .insert(path.clone(), sprite_id);
        asset_manager
            .sprite_id_to_path
            .insert(sprite_id, path.clone());
        asset_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
        asset_manager.enable_runtime_texture_loading_for_test();
        asset_manager.queue_runtime_texture_read(sprite_id);

        assert!(asset_manager.has_pending_texture_read(sprite_id));
        assert_eq!(asset_manager.texture_count(), 0);
        let _ = fs::remove_file(full_path);
    }

    #[test]
    fn poll_pending_runtime_texture_reads_uploads_bytes_on_the_main_thread() {
        let loader = CountingFailingLoader::new();
        let mut asset_manager = AssetManager::default();
        let file_read_pool = FileReadPool::new();
        let path = PathBuf::from(format!(
            "textures/runtime-upload-{}.bin",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX_EPOCH")
                .as_nanos()
        ));
        let sprite_id = SpriteId(7);
        let full_path = assets_folder().join(&path);
        let expected_bytes = [1, 2, 3, 4];

        fs::create_dir_all(
            full_path
                .parent()
                .expect("runtime upload test path should have a parent"),
        )
        .expect("runtime upload test directory should be writable");
        fs::write(&full_path, expected_bytes).expect("runtime upload test file should be writable");

        asset_manager
            .path_to_sprite_id
            .insert(path.clone(), sprite_id);
        asset_manager
            .sprite_id_to_path
            .insert(sprite_id, path.clone());
        asset_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
        asset_manager.enable_runtime_texture_loading_for_test();
        asset_manager.queue_runtime_texture_read(sprite_id);

        // Drain until the read completes and the upload path is hit.
        for _ in 0..100 {
            asset_manager.poll_pending_texture_reads(&loader);
            if !asset_manager.has_pending_texture_read(sprite_id) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(loader.bytes_load_calls.get(), 1);
        assert!(!asset_manager.has_pending_texture_read(sprite_id));
        let _ = fs::remove_file(full_path);
    }
}
