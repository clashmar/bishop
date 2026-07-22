use super::*;

impl SpriteManager {
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
}
