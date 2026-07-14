use bishop::prelude::{vec2, Vec2};
use engine_core::ecs::{Collider, ColliderShape, Ecs, Entity, Transform};
use engine_core::rendering::resolve_visual_entity;
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
    /// The entity that owns the Collider component.
    pub entity: Option<Entity>,
    /// The entity whose Transform is used for positioning.
    pub transform_entity: Option<Entity>,
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
        transform_entity: Entity,
        action: HandleAction,
        initial_collider: Collider,
        drag_start: Vec2,
    ) {
        self.dragging = true;
        self.entity = Some(entity);
        self.transform_entity = Some(transform_entity);
        self.action = Some(action);
        self.initial_collider = Some(initial_collider);
        self.drag_start = drag_start;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Checks if a collider handle was hit on the selected entity.
pub(crate) fn selected_collider_handle_hit(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    mouse_world: Vec2,
) -> Option<(Entity, HandleAction, Collider)> {
    let entity = selected_entity?;
    let visual_entity = resolve_visual_entity(ecs, entity);
    let collider = *ecs.get_store::<Collider>().get(visual_entity)?;
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, transform.pivot, &collider);
    let index = hit_test_handles(mouse_world, &handles)?;

    Some((visual_entity, handles[index].action, collider))
}

/// Nudges the collider offset for the selected entity.
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

    let visual_entity = resolve_visual_entity(ecs, entity);
    let old_collider = *ecs.get_store::<Collider>().get(visual_entity)?;
    let mut new_collider = old_collider;
    new_collider.offset += step;
    Some((visual_entity, old_collider, new_collider))
}

pub(crate) fn apply_handle_drag(
    drag: &ColliderHandleDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
) {
    let delta = mouse_world - drag.drag_start;
    let (Some(entity), Some(transform_entity), Some(initial), Some(action)) = (
        drag.entity,
        drag.transform_entity,
        drag.initial_collider,
        drag.action,
    )
    else {
        return;
    };

    let transform = ecs.get_store::<Transform>().get(transform_entity).copied();
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
    if selected_entity.is_some_and(|e| is_collider_edit_active_for(resolve_visual_entity(ecs, e))) {
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
    let visual_entity = resolve_visual_entity(ecs, entity);
    let collider = ecs.get_store::<Collider>().get(visual_entity)?;
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, transform.pivot, collider);
    let index = hit_test_handles(mouse_world, &handles)?;
    Some((visual_entity, handles[index].action, *collider))
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
#[path = "tests/collider_drag_tests.rs"]
mod tests;
