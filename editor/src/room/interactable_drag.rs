use bishop::prelude::{Vec2, vec2};
use engine_core::ecs::{Ecs, Entity, Interactable, InteractableShape, Transform};
use engine_core::worlds::{RoomId, RoomLayer};

use crate::app::EditorMode;
use crate::commands::scene::{ComponentTransientState, UpdateComponentCmd};
use crate::gui::inspector::interactable_module::edit::{
    compute_handles,
    is_interactable_edit_active_for,
};
use crate::room::bounds_edit::{BoundsEditConfig, HandleAction, hit_test_handles, snap_rect_delta};
use crate::room::selection::can_select_entity_in_room_layer;
use crate::world::coord::round_to_grid;

#[derive(Default)]
pub(crate) struct InteractableHandleDragState {
    /// Whether an interactable handle drag is currently active.
    pub dragging: bool,
    /// The entity that owns the Interactable component.
    pub entity: Option<Entity>,
    /// The entity whose Transform is used for positioning.
    pub transform_entity: Option<Entity>,
    /// The action of the interactable handle currently being dragged.
    pub action: Option<HandleAction>,
    /// The initial interactable state before the handle drag began.
    pub initial_interactable: Option<Interactable>,
    /// Mouse world position at the start of the interactable handle drag.
    pub drag_start: Vec2,
}

impl InteractableHandleDragState {
    pub fn begin(
        &mut self,
        entity: Entity,
        transform_entity: Entity,
        action: HandleAction,
        initial_interactable: Interactable,
        drag_start: Vec2,
    ) {
        self.dragging = true;
        self.entity = Some(entity);
        self.transform_entity = Some(transform_entity);
        self.action = Some(action);
        self.initial_interactable = Some(initial_interactable);
        self.drag_start = drag_start;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Checks if an interactable handle was hit on the selected entity.
pub(crate) fn selected_interactable_handle_hit(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    mouse_world: Vec2,
    grid_size: f32,
) -> Option<(Entity, HandleAction, Interactable)> {
    let entity = selected_entity?;
    let interactable = ecs.get_store::<Interactable>().get(entity)?.clone();
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, &interactable, grid_size);
    let index = hit_test_handles(mouse_world, &handles)?;

    Some((entity, handles[index].action, interactable))
}

/// Nudges the interactable offset for the selected entity.
pub(crate) fn selected_interactable_edit_nudge(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    step: Vec2,
) -> Option<(Entity, Interactable, Interactable)> {
    let entity = selected_entity?;
    if !can_select_entity_in_room_layer(ecs, entity, room_id, layer) {
        return None;
    }

    let old_interactable = ecs.get_store::<Interactable>().get(entity)?.clone();
    let mut new_interactable = old_interactable.clone();
    new_interactable.offset += step;
    Some((entity, old_interactable, new_interactable))
}

pub(crate) fn apply_handle_drag(
    drag: &InteractableHandleDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
    config: BoundsEditConfig,
) {
    let delta = mouse_world - drag.drag_start;
    let (Some(entity), Some(transform_entity), Some(initial), Some(action)) = (
        drag.entity,
        drag.transform_entity,
        drag.initial_interactable.clone(),
        drag.action,
    ) else {
        return;
    };

    let transform = ecs.get_store::<Transform>().get(transform_entity).copied();
    if let (Some(transform), Some(interactable)) = (
        transform,
        ecs.get_store_mut::<Interactable>().get_mut(entity),
    ) {
        *interactable = initial.clone();
        match action {
            HandleAction::MoveOffset => {
                let target = initial.offset + delta;
                if config.snap_enabled {
                    let world_x = transform.position.x + target.x;
                    let world_y = transform.position.y + target.y;
                    interactable.offset.x = round_to_grid(world_x, config.grid_size) - transform.position.x;
                    interactable.offset.y = round_to_grid(world_y, config.grid_size) - transform.position.y;
                } else {
                    interactable.offset = target;
                }
            }
            HandleAction::ResizeAabbTopLeft
            | HandleAction::ResizeAabbTopRight
            | HandleAction::ResizeAabbBottomLeft
            | HandleAction::ResizeAabbBottomRight
            | HandleAction::ResizeTop
            | HandleAction::ResizeBottom
            | HandleAction::ResizeLeft
            | HandleAction::ResizeRight => {
                let snapped_delta = if config.snap_enabled {
                    snap_rect_delta(initial.bounds_at(transform.position), action, delta, config.grid_size)
                } else {
                    delta
                };
                let resized = if config.shift_held {
                    resized_rect_interactable_uniform(initial.clone(), transform, action, snapped_delta)
                } else {
                    resized_rect_interactable(initial.clone(), transform, action, snapped_delta)
                };
                if let Some(resized) = resized {
                    *interactable = resized;
                }
            }
            HandleAction::ResizeCircleRadius => {
                let snapped_mouse = if config.snap_enabled {
                    vec2(
                        round_to_grid(mouse_world.x, config.grid_size),
                        round_to_grid(mouse_world.y, config.grid_size),
                    )
                } else {
                    mouse_world
                };
                if let Some(resized) =
                    resized_circle_interactable(initial.clone(), transform, drag.drag_start, snapped_mouse)
                {
                    *interactable = resized;
                }
            }
            HandleAction::ResizeCapsuleRadiusLeft
            | HandleAction::ResizeCapsuleRadiusRight
            | HandleAction::ResizeCapsuleHeightTop
            | HandleAction::ResizeCapsuleHeightBottom => {}
        }
    }
}

pub(crate) fn finished_interactable_change(
    drag: &InteractableHandleDragState,
    ecs: &Ecs,
) -> Option<(Entity, Interactable, Interactable)> {
    let entity = drag.entity?;
    let old_interactable = drag.initial_interactable.clone()?;
    let new_interactable = ecs.get_store::<Interactable>().get(entity)?.clone();
    if interactables_equal(&old_interactable, &new_interactable) {
        return None;
    }

    Some((entity, old_interactable, new_interactable))
}

/// Result of stepping an active interactable handle drag.
pub(crate) struct InteractableDragStep {
    /// Whether the drag consumed input this frame.
    pub consumed: bool,
    /// Finished change to commit, if the drag ended this frame.
    pub commit: Option<(Entity, Interactable, Interactable)>,
}

/// Advances the active interactable handle drag by one frame.
pub(crate) fn step_active_interactable_drag(
    drag: &mut InteractableHandleDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
    mouse_down: bool,
    mouse_released: bool,
    config: BoundsEditConfig,
) -> InteractableDragStep {
    if !drag.dragging {
        return InteractableDragStep {
            consumed: false,
            commit: None,
        };
    }

    if mouse_down {
        apply_handle_drag(drag, ecs, mouse_world, config);
    }

    let commit = if mouse_released {
        let result = finished_interactable_change(drag, ecs);
        drag.clear();
        result
    } else {
        None
    };

    InteractableDragStep {
        consumed: true,
        commit,
    }
}

/// Checks if an interactable handle was clicked on the currently selected entity.
pub(crate) fn try_intercept_interactable_handle(
    selected_entity: Option<Entity>,
    ecs: &Ecs,
    mouse_world: Vec2,
    grid_size: f32,
) -> Option<(Entity, HandleAction, Interactable)> {
    if selected_entity.is_some_and(is_interactable_edit_active_for) {
        selected_interactable_handle_hit(selected_entity, ecs, mouse_world, grid_size)
    } else {
        None
    }
}

/// Checks if a handle was clicked on a specific entity that has an interactable.
pub(crate) fn try_start_interactable_handle_on_click(
    entity: Entity,
    ecs: &Ecs,
    mouse_world: Vec2,
    grid_size: f32,
) -> Option<(Entity, HandleAction, Interactable)> {
    let interactable = ecs.get_store::<Interactable>().get(entity)?.clone();
    let transform = ecs.get_store::<Transform>().get(entity)?;
    let handles = compute_handles(transform.position, &interactable, grid_size);
    let index = hit_test_handles(mouse_world, &handles)?;
    Some((entity, handles[index].action, interactable))
}

/// Creates an undo command for an interactable change.
pub(crate) fn interactable_update_command(
    entity: Entity,
    old_interactable: Interactable,
    new_interactable: Interactable,
    room_id: RoomId,
) -> Box<UpdateComponentCmd> {
    let old_ron = ron::to_string(&old_interactable).expect("Interactable RON serialize");
    let new_ron = ron::to_string(&new_interactable).expect("Interactable RON serialize");
    Box::new(UpdateComponentCmd::new(
        entity,
        EditorMode::Room(room_id),
        Interactable::TYPE_NAME,
        old_ron,
        new_ron,
        ComponentTransientState::None,
        ComponentTransientState::None,
    ))
}

/// Applies an interactable edit nudge and returns the undo command, or None if not applicable.
pub(crate) fn apply_interactable_edit_nudge(
    selected_entity: Option<Entity>,
    ecs: &mut Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    step: Vec2,
) -> Option<Box<UpdateComponentCmd>> {
    let (entity, old_interactable, new_interactable) =
        selected_interactable_edit_nudge(selected_entity, ecs, room_id, layer, step)?;
    if let Some(interactable) = ecs.get_store_mut::<Interactable>().get_mut(entity) {
        *interactable = new_interactable.clone();
    }
    Some(interactable_update_command(
        entity,
        old_interactable,
        new_interactable,
        room_id,
    ))
}

pub(crate) fn resized_rect_interactable(
    initial: Interactable,
    transform: Transform,
    action: HandleAction,
    delta: Vec2,
) -> Option<Interactable> {
    let InteractableShape::Rect = initial.shape() else {
        return None;
    };

    let bounds = initial.bounds_at(transform.position);
    let min_size = 1.0;
    let left = bounds.x;
    let top = bounds.y;
    let right = bounds.x + bounds.w;
    let bottom = bounds.y + bounds.h;

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
        HandleAction::ResizeTop => (
            left,
            (top + delta.y).min(bottom - min_size),
            right,
            bottom,
        ),
        HandleAction::ResizeBottom => (
            left,
            top,
            right,
            (bottom + delta.y).max(top + min_size),
        ),
        HandleAction::ResizeLeft => (
            (left + delta.x).min(right - min_size),
            top,
            right,
            bottom,
        ),
        HandleAction::ResizeRight => (
            left,
            top,
            (right + delta.x).max(left + min_size),
            bottom,
        ),
        _ => return None,
    };

    let new_width = new_right - new_left;
    let new_height = new_bottom - new_top;
    let new_center = vec2((new_left + new_right) * 0.5, (new_top + new_bottom) * 0.5);
    let mut resized = initial;
    resized.offset = new_center - transform.position;
    resized.rect_size = vec2(new_width, new_height);
    Some(resized)
}

pub(crate) fn resized_rect_interactable_uniform(
    initial: Interactable,
    transform: Transform,
    action: HandleAction,
    delta: Vec2,
) -> Option<Interactable> {
    let InteractableShape::Rect = initial.shape() else {
        return None;
    };

    let bounds = initial.bounds_at(transform.position);
    let size = vec2(bounds.w, bounds.h);
    let center = vec2(bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5);
    let min_size = 1.0;

    let delta_magnitude = match action {
        HandleAction::ResizeAabbTopLeft => (-delta.x).max(-delta.y),
        HandleAction::ResizeAabbTopRight => delta.x.max(-delta.y),
        HandleAction::ResizeAabbBottomLeft => (-delta.x).max(delta.y),
        HandleAction::ResizeAabbBottomRight => delta.x.max(delta.y),
        HandleAction::ResizeTop => -delta.y,
        HandleAction::ResizeBottom => delta.y,
        HandleAction::ResizeLeft => -delta.x,
        HandleAction::ResizeRight => delta.x,
        _ => return None,
    };

    let new_half = (size.x.max(size.y) * 0.5 + delta_magnitude).max(min_size * 0.5);
    let new_size = new_half * 2.0;
    let mut resized = initial;
    resized.offset = center - transform.position;
    resized.rect_size = Vec2::splat(new_size);
    Some(resized)
}

pub(crate) fn resized_circle_interactable(
    initial: Interactable,
    transform: Transform,
    drag_start: Vec2,
    mouse_world: Vec2,
) -> Option<Interactable> {
    let InteractableShape::Circle = initial.shape() else {
        return None;
    };

    let center = initial.center_at(transform.position);
    let start_len = (drag_start - center).length();
    let new_len = (mouse_world - center).length();
    let mut resized = initial;
    resized.radius = (resized.radius + (new_len - start_len)).max(1.0);
    Some(resized)
}

fn interactables_equal(a: &Interactable, b: &Interactable) -> bool {
    a.use_rect == b.use_rect
        && a.offset == b.offset
        && a.radius == b.radius
        && a.rect_size == b.rect_size
}

#[cfg(test)]
#[path = "tests/interactable_drag_tests.rs"]
mod tests;
