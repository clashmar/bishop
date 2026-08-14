use bishop::prelude::*;
use engine_core::ecs::{Collider, ColliderShape, Entity, Pivot};
use engine_core::rendering::pivot_adjusted_position;
use std::cell::Cell;

use crate::gui::inspector::interactable_module::edit::clear_interactable_edit;
pub(crate) use crate::room::bounds_edit::BoundsEditConfig as ColliderEditConfig;
pub(crate) use crate::room::bounds_edit::{Handle, HandleAction, hit_test_handles};
use crate::room::bounds_edit::{compute_circle_handles, compute_rect_handles};

thread_local! {
    static EDIT_ENTITY: Cell<Entity> = const { Cell::new(Entity(0)) };
}

/// Returns the entity currently using collider edit mode.
pub fn collider_edit_entity() -> Option<Entity> {
    EDIT_ENTITY.with(|entity| {
        let entity = entity.get();
        (entity != Entity::null()).then_some(entity)
    })
}

/// Returns true when the given entity owns collider edit mode.
pub fn is_collider_edit_active_for(entity: Entity) -> bool {
    collider_edit_entity() == Some(entity)
}

/// Toggles collider edit mode ownership for an entity.
pub fn toggle_collider_edit(entity: Entity) -> bool {
    clear_interactable_edit(entity);
    EDIT_ENTITY.with(|active| {
        let next = if active.get() == entity {
            Entity::null()
        } else {
            entity
        };
        active.set(next);
        next == entity
    })
}

/// Clears collider edit mode when the given entity owns it.
pub fn clear_collider_edit(entity: Entity) {
    EDIT_ENTITY.with(|active| {
        if active.get() == entity {
            active.set(Entity::null());
        }
    });
}

/// Clears collider edit mode regardless of which entity owns it.
pub fn clear_active_collider_edit() {
    EDIT_ENTITY.with(|active| active.set(Entity::null()));
}

/// Computes handle positions for the given collider shape.
pub fn compute_handles(
    transform_position: Vec2,
    pivot: Pivot,
    collider: &Collider,
    grid_size: f32,
) -> Vec<Handle> {
    let (w, h) = collider.shape.size();
    let size = vec2(w, h);
    let top_left = pivot_adjusted_position(transform_position + collider.offset, size, pivot);
    match collider.shape {
        ColliderShape::Aabb { .. } => {
            compute_rect_handles(Rect::new(top_left.x, top_left.y, w, h), grid_size)
        }
        ColliderShape::Circle { radius } => {
            let center = vec2(top_left.x + radius, top_left.y + radius);
            compute_circle_handles(center, radius, grid_size)
        }
        ColliderShape::Capsule { radius, height } => {
            let hs = grid_size * 0.1;
            let top_center_y = top_left.y + radius;
            let bottom_center_y = top_left.y + radius + height;
            let center_x = top_left.x + w * 0.5;
            let center_y = (top_center_y + bottom_center_y) * 0.5;
            vec![
                Handle {
                    rect: Rect::new(top_left.x - hs, center_y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCapsuleRadiusLeft,
                },
                Handle {
                    rect: Rect::new(top_left.x + w - hs, center_y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCapsuleRadiusRight,
                },
                Handle {
                    rect: Rect::new(center_x - hs, top_left.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCapsuleHeightTop,
                },
                Handle {
                    rect: Rect::new(
                        center_x - hs,
                        top_left.y + h - hs,
                        hs * 2.0,
                        hs * 2.0,
                    ),
                    action: HandleAction::ResizeCapsuleHeightBottom,
                },
                Handle {
                    rect: Rect::new(center_x - hs, center_y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::MoveOffset,
                },
            ]
        }
        ColliderShape::Point => {
            let hs = grid_size * 0.1;
            vec![Handle {
                rect: Rect::new(
                    transform_position.x + collider.offset.x - hs,
                    transform_position.y + collider.offset.y - hs,
                    hs * 2.0,
                    hs * 2.0,
                ),
                action: HandleAction::MoveOffset,
            }]
        }
    }
}
