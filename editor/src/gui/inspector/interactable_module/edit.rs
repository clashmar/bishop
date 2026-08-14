use bishop::prelude::Vec2;
use engine_core::ecs::{Entity, Interactable, InteractableShape};
use std::cell::Cell;

use crate::gui::inspector::collider_module::edit::clear_collider_edit;
use crate::room::bounds_edit::{Handle, compute_circle_handles, compute_rect_handles};

thread_local! {
    static EDIT_ENTITY: Cell<Entity> = const { Cell::new(Entity(0)) };
}

/// Returns the entity currently using interactable edit mode.
pub fn interactable_edit_entity() -> Option<Entity> {
    EDIT_ENTITY.with(|entity| {
        let entity = entity.get();
        (entity != Entity::null()).then_some(entity)
    })
}

/// Returns true when the given entity owns interactable edit mode.
pub fn is_interactable_edit_active_for(entity: Entity) -> bool {
    interactable_edit_entity() == Some(entity)
}

/// Toggles interactable edit mode ownership for an entity.
pub fn toggle_interactable_edit(entity: Entity) -> bool {
    clear_collider_edit(entity);
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

/// Clears interactable edit mode when the given entity owns it.
pub fn clear_interactable_edit(entity: Entity) {
    EDIT_ENTITY.with(|active| {
        if active.get() == entity {
            active.set(Entity::null());
        }
    });
}

/// Clears interactable edit mode regardless of which entity owns it.
pub fn clear_active_interactable_edit() {
    EDIT_ENTITY.with(|active| active.set(Entity::null()));
}

/// Computes handle positions for the given interactable shape.
pub fn compute_handles(
    transform_position: Vec2,
    interactable: &Interactable,
    grid_size: f32,
) -> Vec<Handle> {
    match interactable.shape() {
        InteractableShape::Circle => {
            compute_circle_handles(interactable.center_at(transform_position), interactable.radius, grid_size)
        }
        InteractableShape::Rect => {
            compute_rect_handles(interactable.bounds_at(transform_position), grid_size)
        }
    }
}
