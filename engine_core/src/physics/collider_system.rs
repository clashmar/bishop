// engine_core/src/physics/collider_system.rs
use crate::animation::animation_system::CurrentFrame;
use crate::assets::asset_manager::AssetManager;
use crate::assets::sprite::Sprite;
use crate::assets::sprite::SpriteId;
use crate::ecs::component::Collider;
use crate::ecs::component::ComponentStore;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;

/// Set the collider for every entity that has a sprite and an unset collider
pub fn update_colliders_from_sprites(ecs: &mut Ecs, assets: &mut AssetManager) {
    let mut pending: Vec<(Entity, Collider)> = Vec::new();

    {
        // Immutable access to the two stores.
        let sprite_store = ecs.get_store::<Sprite>();
        let current_frame_store = ecs.get_store::<CurrentFrame>();
        let collider_store = ecs.get_store::<Collider>();

        // Only update entities with colliders
        for (entity, collider) in collider_store.data.iter() {
            if collider.width != 0.0 || collider.height != 0.0 {
                continue;
            }

            // Try animation components first
            if let Some(col) =
                collider_from_animation_component(current_frame_store, *entity, assets)
            {
                pending.push((*entity, col));
                continue; // Found
            }

            // Then try sprite components if not
            for (entity, sprite) in sprite_store.data.iter() {
                if let Some(col) = collider_from_sprite(assets, sprite.sprite) {
                    pending.push((*entity, col));
                }
            }
        }
    }

    // Mutate the Collider store
    if pending.is_empty() {
        return;
    }

    let collider_store = ecs.get_store_mut::<Collider>();

    for (entity, col) in pending {
        if let Some(collider) = collider_store.get_mut(entity) {
            *collider = col;
        }
    }
}

/// Returns a Collider whose dimensions match the sprite size.
pub fn collider_from_sprite(
    asset_manager: &mut AssetManager,
    sprite_id: SpriteId,
) -> Option<Collider> {
    asset_manager
        .texture_size(sprite_id)
        .map(|(w, h)| Collider {
            width: w,
            height: h,
        })
}

/// Try to build a collider from an Animation component.
fn collider_from_animation_component(
    current_frame_store: &ComponentStore<CurrentFrame>,
    entity: Entity,
    asset_manager: &mut AssetManager,
) -> Option<Collider> {
    let current_frame = current_frame_store.get(entity)?;

    // Build the collider
    asset_manager
        .texture_size(current_frame.sprite_id)
        .map(|(_, h)| Collider {
            width: current_frame.frame_size.x,
            height: h,
        })
}
