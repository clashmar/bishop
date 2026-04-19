use super::*;

impl SpriteManager {
    /// Initialize all assets for the game.
    pub fn init_manager(loader: &impl TextureLoader, game: &mut Game) {
        Self::init_editor_metadata(&game.asset_registry, &mut game.sprite_manager);

        let sprites: Vec<(SpriteId, PathBuf)> = game
            .sprite_manager
            .sprite_id_to_path
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect();

        for (id, path) in sprites {
            let _ = game.sprite_manager.reload_texture(loader, &id, &path);
        }

        for animation in game.ecs.get_store_mut::<Animation>().data.values_mut() {
            animation.init_sprite_cache(loader, &mut game.asset_registry, &mut game.sprite_manager);
            animation.init_runtime();
        }
    }

    /// Initialize editor asset metadata without hydrating textures.
    pub fn init_editor_metadata(
        asset_registry: &AssetRegistry,
        sprite_manager: &mut SpriteManager,
    ) {
        sprite_manager.rebuild_path_cache_from_registry(asset_registry);
        sprite_manager.restore_next_sprite_id();
        sprite_manager.restore_next_tile_def_id();
        sprite_manager.runtime_texture_loading = false;
        sprite_manager.runtime_file_read_pool = None;
        sprite_manager.pending_texture_reads.clear();
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

    pub(super) fn rebuild_path_cache_from_registry(&mut self, asset_registry: &AssetRegistry) {
        self.sprite_id_to_path.clear();
        self.path_to_sprite_id.clear();

        for record_key in asset_registry.records().keys().copied() {
            let crate::assets::AssetKey::Sprite(sprite_id) = record_key else {
                continue;
            };
            let Some(relative_path) = asset_registry.relative_path(sprite_id) else {
                continue;
            };

            self.path_to_sprite_id
                .insert(relative_path.clone(), sprite_id);
            self.sprite_id_to_path.insert(sprite_id, relative_path);
        }
    }

    pub(super) fn restore_next_tile_def_id(&mut self) {
        if let Some(max_id) = self.tile_defs.keys().map(|id| id.0).max() {
            self.next_tile_def_id = max_id + 1;
        } else {
            self.next_tile_def_id = 1;
        }
    }
}
