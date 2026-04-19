use super::*;

impl SpriteManager {
    /// Returns the number of tile definitions.
    pub fn tile_def_count(&self) -> usize {
        self.tile_defs.len()
    }

    /// Changes a sprite reference.
    pub fn change_sprite(&mut self, old_id: &mut SpriteId, new_id: SpriteId) {
        if *old_id == new_id {
            return;
        }

        *old_id = new_id;
    }

    /// Changes an optional sprite reference.
    pub fn change_sprite_option(
        &mut self,
        old_id: &mut Option<SpriteId>,
        new_id: Option<SpriteId>,
    ) {
        if *old_id == new_id {
            return;
        }

        *old_id = new_id;
    }

    /// Inserts a TileDef and returns its id.
    pub fn insert_tile_def(&mut self, def: TileDef) -> TileDefId {
        let id = TileDefId(self.next_tile_def_id);
        self.next_tile_def_id += 1;
        self.tile_defs.insert(id, def);
        id
    }

    /// Deletes a TileDef by id.
    pub fn delete_tile_def(&mut self, id: TileDefId) {
        self.tile_defs.remove(&id);
    }

    /// Updates a TileDef's sprite.
    pub fn update_tile_def_sprite(&mut self, id: TileDefId, new_sprite_id: SpriteId) {
        if let Some(def) = self.tile_defs.get_mut(&id)
            && def.sprite_id != new_sprite_id
        {
            def.sprite_id = new_sprite_id;
        }
    }
}
