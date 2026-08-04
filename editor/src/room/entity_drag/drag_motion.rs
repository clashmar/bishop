use super::state::{EntityDragCommand, EntityDragState};
use crate::room::selection::snap_room_drag_position;
use bishop::prelude::Vec2;
use engine_core::ecs::{Ecs, Entity, Pivot, Transform, update_entity_position};
use std::collections::HashSet;

/// Initializes drag state for the current entity selection.
pub(crate) fn begin_entity_drag(
    drag_state: &mut EntityDragState,
    selected_entities: &HashSet<Entity>,
    anchor_entity: Entity,
    ecs: &Ecs,
    mouse_world: Vec2,
) {
    drag_state.dragging = true;
    drag_state.anchor_entity = Some(anchor_entity);
    drag_state.drag_offset = ecs
        .get_store::<Transform>()
        .get(anchor_entity)
        .map(|transform| transform.position - mouse_world)
        .unwrap_or(Vec2::ZERO);
    drag_state.drag_start_positions = selected_entities
        .iter()
        .filter_map(|entity| {
            ecs.get_store::<Transform>()
                .get(*entity)
                .map(|transform| (*entity, transform.position))
        })
        .collect();
    drag_state.drag_initial_start_positions = drag_state.drag_start_positions.clone();
}

/// Advances the active entity drag and optionally builds a commit command.
pub(crate) fn step_active_entity_drag(
    drag_state: &mut EntityDragState,
    ecs: &mut Ecs,
    mouse_world: Vec2,
    mouse_released: bool,
    snap_enabled: bool,
    grid_size: f32,
) -> Option<EntityDragCommand> {
    let Some(anchor_entity) = drag_state.anchor_entity else {
        drag_state.clear();
        return None;
    };
    let Some(anchor_start) = drag_state
        .drag_start_positions
        .iter()
        .find(|(entity, _)| *entity == anchor_entity)
        .map(|(_, position)| *position)
    else {
        drag_state.clear();
        return None;
    };

    let target = if snap_enabled {
        let pivot = ecs
            .get_store::<Transform>()
            .get(anchor_entity)
            .map(|transform| transform.pivot)
            .unwrap_or(Pivot::BottomCenter);
        snap_room_drag_position(mouse_world, grid_size, pivot)
    } else {
        mouse_world + drag_state.drag_offset
    };

    let delta = target - anchor_start;
    for &(entity, start_position) in &drag_state.drag_start_positions {
        update_entity_position(ecs, entity, start_position + delta);
    }

    if mouse_released {
        finish_entity_drag(drag_state, ecs)
    } else {
        None
    }
}

/// Builds the final move command for a completed entity drag.
pub(crate) fn finish_entity_drag(
    drag_state: &mut EntityDragState,
    ecs: &Ecs,
) -> Option<EntityDragCommand> {
    let mut moves = Vec::new();
    for &(entity, initial_position) in &drag_state.drag_initial_start_positions {
        let final_position = match ecs.get_store::<Transform>().get(entity) {
            Some(transform) => transform.position,
            None => {
                drag_state.clear();
                return None;
            }
        };
        if (final_position - initial_position).length_squared() > 0.0 {
            moves.push((entity, initial_position, final_position));
        }
    }

    let commit = match moves.len() {
        0 => None,
        1 => {
            let (entity, from, to) = moves[0];
            Some(EntityDragCommand::MoveOne { entity, from, to })
        }
        _ => Some(EntityDragCommand::MoveMany { moves }),
    };
    drag_state.clear();
    commit
}
