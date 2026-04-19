use super::*;

impl SpriteManager {
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
