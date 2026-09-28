use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::component::ComponentStore;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{Collider, ColliderData, ColliderFrameKey, ColliderShape, CurrentFrame, Sprite, SpriteId};


/// Returns the collider data effective for an entity's current frame.
pub fn collider_data_for_entity(ecs: &Ecs, entity: Entity) -> Option<ColliderData> {
    let collider = ecs.get_store::<Collider>().get(entity)?;
    let Some(current_frame) = ecs.get_store::<CurrentFrame>().get(entity) else {
        return Some(collider.static_data());
    };

    Some(collider.effective_data_for_frame(
        &current_frame.clip_id,
        ColliderFrameKey {
            row: current_frame.row,
            col: current_frame.col,
        },
    ))
}

/// Set the collider for every entity that has a sprite and an unset collider
pub fn update_colliders_from_sprites(ecs: &mut Ecs, assets: &mut SpriteManager) {
    let mut pending: Vec<(Entity, Collider)> = Vec::new();

    {
        // Immutable access to the two stores.
        let sprite_store = ecs.get_store::<Sprite>();
        let current_frame_store = ecs.get_store::<CurrentFrame>();
        let collider_store = ecs.get_store::<Collider>();

        // Only update entities with colliders
        for (entity, collider) in collider_store.data.iter() {
            if !collider.shape.is_default_size() {
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
    sprite_manager: &mut SpriteManager,
    sprite_id: SpriteId,
) -> Option<Collider> {
    sprite_manager
        .texture_size(sprite_id)
        .map(|(w, h)| Collider {
            shape: ColliderShape::Aabb { width: w, height: h },
            ..Default::default()
        })
}

/// Try to build a collider from an Animation component.
pub fn collider_from_animation_component(
    current_frame_store: &ComponentStore<CurrentFrame>,
    entity: Entity,
    sprite_manager: &mut SpriteManager,
) -> Option<Collider> {
    let current_frame = current_frame_store.get(entity)?;

    // Build the collider
    sprite_manager
        .texture_size(current_frame.sprite_id)
        .map(|(_, h)| Collider {
            shape: ColliderShape::Aabb {
                width: current_frame.frame_size.x,
                height: h,
            },
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::ClipId;
    use bishop::prelude::Vec2;

    #[test]
    fn collider_data_for_entity_uses_current_frame_when_present() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Collider::default())
            .finish();
        let frame = ColliderFrameKey { row: 0, col: 1 };
        let frame_data = ColliderData {
            shape: ColliderShape::Circle { radius: 12.0 },
            offset: Vec2::new(4.0, 5.0),
        };

        ecs.get_mut::<Collider>(entity)
            .unwrap()
            .set_frame_data(ClipId::Run, frame, frame_data);
        ecs.add_component_to_entity(
            entity,
            CurrentFrame {
                clip_id: ClipId::Run,
                row: frame.row,
                col: frame.col,
                ..Default::default()
            },
        );

        assert_eq!(collider_data_for_entity(&ecs, entity), Some(frame_data));
    }

    #[test]
    fn collider_data_for_entity_falls_back_to_static_without_current_frame() {
        let mut ecs = Ecs::default();
        let collider = Collider {
            shape: ColliderShape::Aabb {
                width: 22.0,
                height: 33.0,
            },
            offset: Vec2::new(2.0, 3.0),
            ..Default::default()
        };
        let static_data = collider.static_data();
        let entity = ecs.create_entity().with(collider).finish();

        assert_eq!(collider_data_for_entity(&ecs, entity), Some(static_data));
    }
}
