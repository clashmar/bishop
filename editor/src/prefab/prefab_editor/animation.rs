use super::selection::is_prefab_entity;
use super::PrefabEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    pub(crate) fn tick_prefab_animations(
        &self,
        loader: &impl TextureLoader,
        ecs: &mut Ecs,
        sprite_manager: &mut SpriteManager,
        dt: f32,
    ) {
        let entities = ecs
            .get_store::<Transform>()
            .data
            .keys()
            .copied()
            .filter(|entity| is_prefab_entity(ecs, *entity))
            .collect();

        update_entity_animations(loader, ecs, sprite_manager, dt, &entities);
    }
}
