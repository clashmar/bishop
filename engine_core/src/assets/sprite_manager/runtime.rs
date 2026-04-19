use super::*;

impl SpriteManager {
    /// Initialize game data for runtime startup without hydrating all textures up front.
    pub fn init_runtime_manager(game: &mut Game) {
        let file_read_pool = FileReadPool::new();
        Self::init_runtime_manager_with_pool(&file_read_pool, game);
    }

    /// Initialize runtime data with an explicit file-read pool handle.
    pub fn init_runtime_manager_with_pool(file_read_pool: &FileReadPool, game: &mut Game) {
        game.sprite_manager
            .rebuild_path_cache_from_registry(&game.asset_registry);
        game.sprite_manager.restore_next_sprite_id();
        game.sprite_manager.restore_next_tile_def_id();
        game.sprite_manager.runtime_texture_loading = true;
        game.sprite_manager.runtime_file_read_pool = Some(file_read_pool.clone());
        game.sprite_manager.pending_texture_reads.clear();

        for animation in game.ecs.get_store_mut::<Animation>().data.values_mut() {
            animation.init_sprite_cache_runtime(&game.sprite_manager);
            animation.init_runtime();
        }
    }

    pub(super) fn queue_runtime_texture_read(&mut self, id: SpriteId) {
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
        file_read_pool.queue_read(id.0.to_string(), full_path.clone());
        self.pending_texture_reads.insert(id, full_path);
    }

    pub(super) fn poll_pending_texture_reads(&mut self, loader: &impl TextureLoader) {
        let Some(file_read_pool) = self.runtime_file_read_pool.clone() else {
            return;
        };

        while let Some(completed) = file_read_pool.try_recv_completed() {
            let Ok(sprite_index) = completed.id.parse::<usize>() else {
                continue;
            };
            let sprite_id = SpriteId(sprite_index);

            let Some(abs_path) = self.pending_texture_reads.get(&sprite_id).cloned() else {
                continue;
            };

            if completed.path != abs_path {
                continue;
            }

            self.pending_texture_reads.remove(&sprite_id);
            let path_display = abs_path.display().to_string();

            match completed.result {
                Ok(bytes) => match loader.load_texture_from_bytes(&bytes) {
                    Ok(texture) => {
                        self.textures.insert(sprite_id, texture);
                        if let Some(rel_path) = self.sprite_id_to_path.get(&sprite_id).cloned() {
                            self.path_to_sprite_id.insert(rel_path, sprite_id);
                        }
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
    pub(super) fn enable_runtime_texture_loading_for_test(&mut self) {
        self.runtime_texture_loading = true;
    }

    #[cfg(test)]
    pub(super) fn attach_runtime_file_read_pool_for_test(&mut self, file_read_pool: &FileReadPool) {
        self.runtime_file_read_pool = Some(file_read_pool.clone());
    }

    #[cfg(test)]
    pub(super) fn has_pending_texture_read(&self, id: SpriteId) -> bool {
        self.pending_texture_reads.contains_key(&id)
    }
}
