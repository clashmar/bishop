use bishop::prelude::*;
use engine_core::ecs::{Collider, ColliderShape, Entity, Pivot};
use engine_core::rendering::pivot_adjusted_position;
use std::cell::Cell;

thread_local! {
    static EDIT_ENTITY: Cell<Entity> = const { Cell::new(Entity(0)) };
}

/// Configuration passed through the collider drag pipeline.
#[derive(Clone, Copy)]
pub(crate) struct ColliderEditConfig {
    pub grid_size: f32,
    pub snap_enabled: bool,
    pub shift_held: bool,
}

/// An interaction handle on the collider outline.
pub struct Handle {
    pub rect: Rect,
    pub action: HandleAction,
}

/// Actions that can be performed by dragging a handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleAction {
    ResizeAabbTopLeft,
    ResizeAabbTopRight,
    ResizeAabbBottomLeft,
    ResizeAabbBottomRight,
    ResizeCircleRadius,
    ResizeCapsuleRadiusLeft,
    ResizeCapsuleRadiusRight,
    ResizeCapsuleHeightTop,
    ResizeCapsuleHeightBottom,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    MoveOffset,
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
    let hs = grid_size * 0.1;

    match collider.shape {
        ColliderShape::Aabb { .. } => {
            vec![
                // Corners
                Handle {
                    rect: Rect::new(top_left.x - hs, top_left.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeAabbTopLeft,
                },
                Handle {
                    rect: Rect::new(top_left.x + w - hs, top_left.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeAabbTopRight,
                },
                Handle {
                    rect: Rect::new(top_left.x - hs, top_left.y + h - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeAabbBottomLeft,
                },
                Handle {
                    rect: Rect::new(top_left.x + w - hs, top_left.y + h - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeAabbBottomRight,
                },
                // Edges
                Handle {
                    rect: Rect::new(top_left.x + w / 2.0 - hs, top_left.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeTop,
                },
                Handle {
                    rect: Rect::new(top_left.x + w / 2.0 - hs, top_left.y + h - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeBottom,
                },
                Handle {
                    rect: Rect::new(top_left.x - hs, top_left.y + h / 2.0 - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeLeft,
                },
                Handle {
                    rect: Rect::new(top_left.x + w - hs, top_left.y + h / 2.0 - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeRight,
                },
                Handle {
                    rect: Rect::new(
                        top_left.x + w / 2.0 - hs,
                        top_left.y + h / 2.0 - hs,
                        hs * 2.0,
                        hs * 2.0,
                    ),
                    action: HandleAction::MoveOffset,
                },
            ]
        }
        ColliderShape::Circle { radius } => {
            let center = vec2(top_left.x + radius, top_left.y + radius);
            vec![
                Handle {
                    rect: Rect::new(center.x + radius - hs, center.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCircleRadius,
                },
                Handle {
                    rect: Rect::new(center.x - radius - hs, center.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCircleRadius,
                },
                Handle {
                    rect: Rect::new(center.x - hs, center.y + radius - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCircleRadius,
                },
                Handle {
                    rect: Rect::new(center.x - hs, center.y - radius - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::ResizeCircleRadius,
                },
                Handle {
                    rect: Rect::new(center.x - hs, center.y - hs, hs * 2.0, hs * 2.0),
                    action: HandleAction::MoveOffset,
                },
            ]
        }
        ColliderShape::Capsule { radius, height } => {
            let top_center_y = top_left.y + radius;
            let bottom_center_y = top_left.y + radius + height;
            let center_x = top_left.x + w / 2.0;
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
        ColliderShape::Point => vec![Handle {
            rect: Rect::new(
                transform_position.x + collider.offset.x - hs,
                transform_position.y + collider.offset.y - hs,
                hs * 2.0,
                hs * 2.0,
            ),
            action: HandleAction::MoveOffset,
        }],
    }
}

/// Returns the index of the handle under the given mouse position, if any.
pub fn hit_test_handles(mouse_pos: Vec2, handles: &[Handle]) -> Option<usize> {
    handles.iter().position(|h| {
        mouse_pos.x >= h.rect.x
            && mouse_pos.x <= h.rect.x + h.rect.w
            && mouse_pos.y >= h.rect.y
            && mouse_pos.y <= h.rect.y + h.rect.h
    })
}