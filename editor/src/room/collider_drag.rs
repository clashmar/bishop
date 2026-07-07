use bishop::prelude::{vec2, Vec2};
use engine_core::ecs::{Collider, ColliderShape, Ecs, Entity, Transform};
use engine_core::worlds::RoomId;

use crate::app::EditorMode;
use crate::commands::scene::{ComponentTransientState, UpdateComponentCmd};
use crate::gui::inspector::collider_module::edit::{
    compute_handles,
    hit_test_handles,
    is_collider_edit_active_for,
    HandleAction,
};
use crate::room::selection::can_select_entity_in_room;

#[derive(Default)]
pub(crate) struct ColliderHandleDragState {
    /// Whether a collider handle drag is currently active.
    pub dragging: bool,
    /// The entity whose collider handle is being dragged.
    pub entity: Option<Entity>,
    /// The action of the collider handle currently being dragged.
    pub action: Option<HandleAction>,
    /// The initial collider state before the handle drag began.
    pub initial_collider: Option<Collider>,
    /// Mouse world position at the start of the collider handle drag.
    pub drag_start: Vec2,
}

impl ColliderHandleDragState {
    pub fn begin(
        &mut self,
        entity: Entity,
        action: HandleAction,
        initial_collider: Collider,
        drag_start: Vec2,
    ) {
        self.dragging = true;
        self.entity = Some(entity);
        self.action = Some(action);
        self.initial_collider = Some(initial_collider);
        self.drag_start = drag_start;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub(crate) fn selected_collider_handle_hit(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    mouse_world: Vec2,
) -> Option<(Entity, HandleAction, Collider)> {
    let entity = selected_entity?;
    let collider = *ecs.get_store::<Collider>().get(entity)?;
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, transform.pivot, &collider);
    let index = hit_test_handles(mouse_world, &handles)?;

    Some((entity, handles[index].action, collider))
}

pub(crate) fn selected_collider_edit_nudge(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    room_id: RoomId,
    step: Vec2,
) -> Option<(Entity, Collider, Collider)> {
    let entity = selected_entity?;
    if !can_select_entity_in_room(ecs, entity, room_id) {
        return None;
    }

    let old_collider = *ecs.get_store::<Collider>().get(entity)?;
    let mut new_collider = old_collider;
    new_collider.offset += step;
    Some((entity, old_collider, new_collider))
}

pub(crate) fn apply_handle_drag(
    drag: &ColliderHandleDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
) {
    let delta = mouse_world - drag.drag_start;
    let (Some(entity), Some(initial), Some(action)) = (drag.entity, drag.initial_collider, drag.action)
    else {
        return;
    };

    let transform = ecs.get_store::<Transform>().get(entity).copied();
    if let (Some(transform), Some(collider)) = (
        transform,
        ecs.get_store_mut::<Collider>().get_mut(entity),
    ) {
        *collider = initial;
        match action {
            HandleAction::MoveOffset => {
                collider.offset = initial.offset + delta;
            }
            HandleAction::ResizeAabbTopLeft
            | HandleAction::ResizeAabbTopRight
            | HandleAction::ResizeAabbBottomLeft
            | HandleAction::ResizeAabbBottomRight => {
                if let Some(resized) = resized_aabb_collider(initial, transform, action, delta) {
                    *collider = resized;
                }
            }
            HandleAction::ResizeCircleRadius => {
                if let Some(resized) =
                    resized_circle_collider(initial, transform, drag.drag_start, mouse_world)
                {
                    *collider = resized;
                }
            }
            HandleAction::ResizeCapsuleRadiusLeft
            | HandleAction::ResizeCapsuleRadiusRight
            | HandleAction::ResizeCapsuleHeightTop
            | HandleAction::ResizeCapsuleHeightBottom => {
                if let Some(resized) = resized_capsule_collider(initial, transform, action, delta)
                {
                    *collider = resized;
                }
            }
        }
    }
}

pub(crate) fn finished_collider_change(
    drag: &ColliderHandleDragState,
    ecs: &Ecs,
) -> Option<(Entity, Collider, Collider)> {
    let entity = drag.entity?;
    let old_collider = drag.initial_collider?;
    let new_collider = *ecs.get_store::<Collider>().get(entity)?;
    if new_collider.shape == old_collider.shape && new_collider.offset == old_collider.offset {
        return None;
    }

    Some((entity, old_collider, new_collider))
}

/// Result of stepping an active collider handle drag.
pub(crate) struct ColliderDragStep {
    /// Whether the drag consumed input this frame.
    pub consumed: bool,
    /// Finished change to commit, if the drag ended this frame.
    pub commit: Option<(Entity, Collider, Collider)>,
}

/// Advances the active collider handle drag by one frame.
pub(crate) fn step_active_collider_drag(
    drag: &mut ColliderHandleDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
    mouse_down: bool,
    mouse_released: bool,
) -> ColliderDragStep {
    if !drag.dragging {
        return ColliderDragStep { consumed: false, commit: None };
    }

    if mouse_down {
        apply_handle_drag(drag, ecs, mouse_world);
    }

    let commit = if mouse_released {
        let result = finished_collider_change(drag, ecs);
        drag.clear();
        result
    } else {
        None
    };

    ColliderDragStep { consumed: true, commit }
}

/// Checks if a collider handle was clicked on the currently selected entity.
pub(crate) fn try_intercept_collider_handle(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    mouse_world: Vec2,
) -> Option<(Entity, HandleAction, Collider)> {
    if selected_entity.is_some_and(is_collider_edit_active_for) {
        selected_collider_handle_hit(selected_entity, ecs, mouse_world)
    } else {
        None
    }
}

/// Checks if a handle was clicked on a specific entity that has a collider.
pub(crate) fn try_start_collider_handle_on_click(
    entity: Entity,
    ecs: &Ecs,
    mouse_world: Vec2,
) -> Option<(Entity, HandleAction, Collider)> {
    let collider = ecs.get_store::<Collider>().get(entity)?;
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, transform.pivot, collider);
    let index = hit_test_handles(mouse_world, &handles)?;
    Some((entity, handles[index].action, *collider))
}

/// Creates an undo command for a collider change.
pub(crate) fn collider_update_command(
    entity: Entity,
    old_collider: Collider,
    new_collider: Collider,
    room_id: RoomId,
) -> Box<UpdateComponentCmd> {
    let old_ron = ron::to_string(&old_collider).expect("Collider RON serialize");
    let new_ron = ron::to_string(&new_collider).expect("Collider RON serialize");
    Box::new(UpdateComponentCmd::new(
        entity,
        EditorMode::Room(room_id),
        Collider::TYPE_NAME,
        old_ron,
        new_ron,
        ComponentTransientState::None,
        ComponentTransientState::None,
    ))
}

/// Applies a collider edit nudge and returns the undo command, or None if not applicable.
pub(crate) fn apply_collider_edit_nudge(
    selected_entity: Option<Entity>,
    ecs: &mut Ecs,
    room_id: RoomId,
    step: Vec2,
) -> Option<Box<UpdateComponentCmd>> {
    let (entity, old_collider, new_collider) =
        selected_collider_edit_nudge(selected_entity, ecs, room_id, step)?;
    if let Some(collider) = ecs.get_store_mut::<Collider>().get_mut(entity) {
        *collider = new_collider;
    }
    Some(collider_update_command(entity, old_collider, new_collider, room_id))
}

pub(crate) fn resized_aabb_collider(
    initial: Collider,
    transform: Transform,
    action: HandleAction,
    delta: Vec2,
) -> Option<Collider> {
    let ColliderShape::Aabb { width, height } = initial.shape else {
        return None;
    };

    let size = vec2(width, height);
    let top_left = engine_core::rendering::pivot_adjusted_position(
        transform.position + initial.offset,
        size,
        transform.pivot,
    );
    let min_size = 1.0;
    let left = top_left.x;
    let top = top_left.y;
    let right = left + width;
    let bottom = top + height;

    let (new_left, new_top, new_right, new_bottom) = match action {
        HandleAction::ResizeAabbTopLeft => (
            (left + delta.x).min(right - min_size),
            (top + delta.y).min(bottom - min_size),
            right,
            bottom,
        ),
        HandleAction::ResizeAabbTopRight => (
            left,
            (top + delta.y).min(bottom - min_size),
            (right + delta.x).max(left + min_size),
            bottom,
        ),
        HandleAction::ResizeAabbBottomLeft => (
            (left + delta.x).min(right - min_size),
            top,
            right,
            (bottom + delta.y).max(top + min_size),
        ),
        HandleAction::ResizeAabbBottomRight => (
            left,
            top,
            (right + delta.x).max(left + min_size),
            (bottom + delta.y).max(top + min_size),
        ),
        _ => return None,
    };

    let new_width = new_right - new_left;
    let new_height = new_bottom - new_top;
    let pivot = transform.pivot.as_normalized();
    let new_offset = vec2(new_left, new_top) - transform.position
        + vec2(new_width * pivot.x, new_height * pivot.y);

    Some(Collider {
        shape: ColliderShape::Aabb {
            width: new_width,
            height: new_height,
        },
        offset: new_offset,
    })
}

pub(crate) fn resized_circle_collider(
    initial: Collider,
    transform: Transform,
    drag_start: Vec2,
    mouse_world: Vec2,
) -> Option<Collider> {
    let ColliderShape::Circle { radius } = initial.shape else {
        return None;
    };

    let size = Vec2::splat(radius * 2.0);
    let top_left = engine_core::rendering::pivot_adjusted_position(
        transform.position + initial.offset,
        size,
        transform.pivot,
    );
    let center = top_left + size * 0.5;
    let start_len = (drag_start - center).length();
    let new_len = (mouse_world - center).length();
    let new_radius = (radius + (new_len - start_len)).max(0.5);
    let new_size = Vec2::splat(new_radius * 2.0);
    let new_top_left = center - new_size * 0.5;
    let pivot = transform.pivot.as_normalized();
    let new_offset = new_top_left - transform.position + new_size * pivot;

    Some(Collider {
        shape: ColliderShape::Circle { radius: new_radius },
        offset: new_offset,
    })
}

pub(crate) fn resized_capsule_collider(
    initial: Collider,
    transform: Transform,
    action: HandleAction,
    delta: Vec2,
) -> Option<Collider> {
    let ColliderShape::Capsule { radius, height } = initial.shape else {
        return None;
    };

    let width = radius * 2.0;
    let total_height = height + width;
    let size = vec2(width, total_height);
    let top_left = engine_core::rendering::pivot_adjusted_position(
        transform.position + initial.offset,
        size,
        transform.pivot,
    );
    let min_width = 1.0;
    let left = top_left.x;
    let top = top_left.y;
    let right = left + width;
    let bottom = top + total_height;

    let (new_left, new_top, new_width, new_total_height) = match action {
        HandleAction::ResizeCapsuleRadiusLeft => {
            let clamped_left = (left + delta.x).min(right - min_width).max(right - total_height);
            (clamped_left, top, right - clamped_left, total_height)
        }
        HandleAction::ResizeCapsuleRadiusRight => {
            let clamped_right = (right + delta.x).max(left + min_width).min(left + total_height);
            (left, top, clamped_right - left, total_height)
        }
        HandleAction::ResizeCapsuleHeightTop => {
            let clamped_top = (top + delta.y).min(bottom - width);
            (left, clamped_top, width, bottom - clamped_top)
        }
        HandleAction::ResizeCapsuleHeightBottom => {
            let clamped_bottom = (bottom + delta.y).max(top + width);
            (left, top, width, clamped_bottom - top)
        }
        _ => return None,
    };

    let new_radius = new_width * 0.5;
    let new_height = new_total_height - new_width;
    let pivot = transform.pivot.as_normalized();
    let new_offset = vec2(new_left, new_top) - transform.position
        + vec2(new_width * pivot.x, new_total_height * pivot.y);

    Some(Collider {
        shape: ColliderShape::Capsule {
            radius: new_radius,
            height: new_height,
        },
        offset: new_offset,
    })
}

#[cfg(test)]
mod tests {
    use bishop::prelude::{vec2, Rect, Vec2};
    use engine_core::ecs::*;
    use engine_core::worlds::RoomId;

    use super::{
        resized_aabb_collider,
        resized_capsule_collider,
        resized_circle_collider,
        selected_collider_edit_nudge,
        selected_collider_handle_hit,
    };

    fn aabb_rect(collider: Collider, transform: Transform) -> Rect {
        let ColliderShape::Aabb { width, height } = collider.shape else {
            panic!("expected AABB collider");
        };
        let top_left = engine_core::rendering::pivot_adjusted_position(
            transform.position + collider.offset,
            vec2(width, height),
            transform.pivot,
        );
        Rect::new(top_left.x, top_left.y, width, height)
    }

    fn circle_center(collider: Collider, transform: Transform) -> Vec2 {
        let ColliderShape::Circle { radius } = collider.shape else {
            panic!("expected circle collider");
        };
        let top_left = engine_core::rendering::pivot_adjusted_position(
            transform.position + collider.offset,
            Vec2::splat(radius * 2.0),
            transform.pivot,
        );
        top_left + Vec2::splat(radius)
    }

    fn capsule_rect(collider: Collider, transform: Transform) -> Rect {
        let ColliderShape::Capsule { radius, height } = collider.shape else {
            panic!("expected capsule collider");
        };
        let size = vec2(radius * 2.0, height + radius * 2.0);
        let top_left = engine_core::rendering::pivot_adjusted_position(
            transform.position + collider.offset,
            size,
            transform.pivot,
        );
        Rect::new(top_left.x, top_left.y, size.x, size.y)
    }

    #[test]
    fn selected_collider_handle_hit_mouse_over_handle_returns_selected_entity_action() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform {
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .with(Collider::default())
            .finish();
        let collider = match ecs.get_store::<Collider>().get(entity) {
            Some(collider) => *collider,
            None => panic!("expected collider on test entity"),
        };
        let handles = crate::gui::inspector::collider_module::edit::compute_handles(
            Vec2::ZERO,
            Pivot::TopLeft,
            &collider,
        );
        let mouse_world = match handles.last() {
            Some(handle) => vec2(
                handle.rect.x + handle.rect.w * 0.5,
                handle.rect.y + handle.rect.h * 0.5,
            ),
            None => panic!("expected collider handles for default collider"),
        };

        let hit = selected_collider_handle_hit(Some(entity), &ecs, mouse_world);

        match hit {
            Some((hit_entity, action, initial_collider)) => {
                assert_eq!(hit_entity, entity);
                assert_eq!(
                    action,
                    crate::gui::inspector::collider_module::edit::HandleAction::MoveOffset,
                );
                assert_eq!(initial_collider.offset, Vec2::ZERO);
            }
            None => panic!("expected selected collider handle hit"),
        }
    }

    #[test]
    fn selected_collider_handle_hit_missing_collider_returns_none() {
        let mut ecs = Ecs::default();
        let entity = ecs.create_entity().with(Transform::default()).finish();

        let hit = selected_collider_handle_hit(Some(entity), &ecs, Vec2::ZERO);

        assert!(hit.is_none());
    }

    #[test]
    fn selected_collider_edit_nudge_in_room_returns_updated_offset() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Collider::default())
            .with_current_room(RoomId(7))
            .finish();

        let nudged = selected_collider_edit_nudge(Some(entity), &ecs, RoomId(7), vec2(1.0, -1.0));

        match nudged {
            Some((nudged_entity, old_collider, new_collider)) => {
                assert_eq!(nudged_entity, entity);
                assert_eq!(old_collider.offset, Vec2::ZERO);
                assert_eq!(new_collider.offset, vec2(1.0, -1.0));
                assert_eq!(new_collider.shape, old_collider.shape);
            }
            None => panic!("expected collider edit nudge"),
        }
    }

    #[test]
    fn resized_aabb_collider_top_right_drag_keeps_bottom_left_fixed() {
        let initial = Collider {
            shape: ColliderShape::Aabb {
                width: 10.0,
                height: 20.0,
            },
            ..Default::default()
        };
        let transform = Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        };
        let initial_rect = aabb_rect(initial, transform);

        let resized = resized_aabb_collider(
            initial,
            transform,
            crate::gui::inspector::collider_module::edit::HandleAction::ResizeAabbTopRight,
            vec2(4.0, -3.0),
        );

        match resized {
            Some(collider) => {
                let resized_rect = aabb_rect(collider, transform);
                assert_eq!(resized_rect.x, initial_rect.x);
                assert_eq!(resized_rect.y + resized_rect.h, initial_rect.y + initial_rect.h);
                assert_eq!(resized_rect.w, 14.0);
                assert_eq!(resized_rect.h, 23.0);
            }
            None => panic!("expected resized collider"),
        }
    }

    #[test]
    fn resized_aabb_collider_top_left_drag_keeps_bottom_right_fixed() {
        let initial = Collider {
            shape: ColliderShape::Aabb {
                width: 10.0,
                height: 20.0,
            },
            ..Default::default()
        };
        let transform = Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        };
        let initial_rect = aabb_rect(initial, transform);

        let resized = resized_aabb_collider(
            initial,
            transform,
            crate::gui::inspector::collider_module::edit::HandleAction::ResizeAabbTopLeft,
            vec2(-4.0, -3.0),
        );

        match resized {
            Some(collider) => {
                let resized_rect = aabb_rect(collider, transform);
                assert_eq!(resized_rect.x + resized_rect.w, initial_rect.x + initial_rect.w);
                assert_eq!(resized_rect.y + resized_rect.h, initial_rect.y + initial_rect.h);
                assert_eq!(resized_rect.w, 14.0);
                assert_eq!(resized_rect.h, 23.0);
            }
            None => panic!("expected resized collider"),
        }
    }

    #[test]
    fn resized_circle_collider_with_bottom_center_pivot_keeps_center_fixed() {
        let initial = Collider {
            shape: ColliderShape::Circle { radius: 5.0 },
            ..Default::default()
        };
        let transform = Transform {
            pivot: Pivot::BottomCenter,
            ..Default::default()
        };
        let initial_center = circle_center(initial, transform);

        let resized = resized_circle_collider(
            initial,
            transform,
            initial_center + vec2(5.0, 0.0),
            initial_center + vec2(8.0, 0.0),
        );

        match resized {
            Some(collider) => {
                assert_eq!(circle_center(collider, transform), initial_center);
                assert_eq!(collider.shape, ColliderShape::Circle { radius: 8.0 });
            }
            None => panic!("expected resized circle collider"),
        }
    }

    #[test]
    fn resized_capsule_collider_right_drag_preserves_total_height() {
        let initial = Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        };
        let transform = Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        };
        let initial_rect = capsule_rect(initial, transform);

        let resized = resized_capsule_collider(
            initial,
            transform,
            crate::gui::inspector::collider_module::edit::HandleAction::ResizeCapsuleRadiusRight,
            vec2(2.0, 0.0),
        );

        match resized {
            Some(collider) => {
                let resized_rect = capsule_rect(collider, transform);
                assert_eq!(resized_rect.x, initial_rect.x);
                assert_eq!(resized_rect.y, initial_rect.y);
                assert_eq!(resized_rect.h, initial_rect.h);
                assert_eq!(resized_rect.w, 10.0);
                assert_eq!(
                    collider.shape,
                    ColliderShape::Capsule {
                        radius: 5.0,
                        height: 8.0,
                    },
                );
            }
            None => panic!("expected resized capsule collider"),
        }
    }
}
