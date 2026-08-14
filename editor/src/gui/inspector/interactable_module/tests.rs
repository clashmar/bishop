use bishop::prelude::{Vec2, vec2};
use engine_core::ecs::{Entity, Interactable, InteractableShape};

use super::edit::{
    clear_active_interactable_edit,
    clear_interactable_edit,
    compute_handles,
    interactable_edit_entity,
    toggle_interactable_edit,
};
use crate::gui::inspector::collider_module::edit::{
    clear_collider_edit,
    is_collider_edit_active_for,
    toggle_collider_edit,
};

#[test]
fn toggle_interactable_edit_claims_and_releases_entity() {
    let entity = Entity(17);
    clear_active_interactable_edit();
    clear_interactable_edit(entity);

    assert!(toggle_interactable_edit(entity));
    assert_eq!(interactable_edit_entity(), Some(entity));
    assert!(!toggle_interactable_edit(entity));
    assert_eq!(interactable_edit_entity(), None);
}

#[test]
fn toggling_interactable_edit_clears_collider_edit_for_same_entity() {
    let entity = Entity(23);
    clear_collider_edit(entity);
    clear_interactable_edit(entity);
    assert!(toggle_collider_edit(entity));

    assert!(toggle_interactable_edit(entity));

    assert_eq!(interactable_edit_entity(), Some(entity));
    assert!(!is_collider_edit_active_for(entity));
}

#[test]
fn toggling_collider_edit_clears_interactable_edit_for_same_entity() {
    let entity = Entity(29);
    clear_collider_edit(entity);
    clear_interactable_edit(entity);
    assert!(toggle_interactable_edit(entity));

    assert!(toggle_collider_edit(entity));

    assert!(is_collider_edit_active_for(entity));
    assert_eq!(interactable_edit_entity(), None);
}

#[test]
fn interactable_rect_handles_include_edge_midpoints() {
    let interactable = Interactable::rect(Vec2::ZERO, vec2(32.0, 16.0));
    let handles = compute_handles(Vec2::ZERO, &interactable, 16.0);
    let actions: Vec<_> = handles.iter().map(|handle| handle.action).collect();

    assert_eq!(interactable.shape(), InteractableShape::Rect);
    assert!(actions.contains(&crate::room::bounds_edit::HandleAction::ResizeTop));
    assert!(actions.contains(&crate::room::bounds_edit::HandleAction::ResizeBottom));
    assert!(actions.contains(&crate::room::bounds_edit::HandleAction::ResizeLeft));
    assert!(actions.contains(&crate::room::bounds_edit::HandleAction::ResizeRight));
}
