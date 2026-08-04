use bishop::prelude::{Rect, Vec2, vec2};
use engine_core::ecs::{Ecs, Interactable, Pivot, Transform};
use engine_core::worlds::{RoomId, RoomLayer};

use super::{
    resized_circle_interactable,
    resized_rect_interactable,
    selected_interactable_edit_nudge,
    selected_interactable_handle_hit,
};
use crate::room::bounds_edit::HandleAction;

fn interactable_rect(interactable: &Interactable, transform: Transform) -> Rect {
    interactable.bounds_at(transform.position)
}

#[test]
fn selected_interactable_handle_hit_mouse_over_handle_returns_selected_entity_action() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Interactable::rect(Vec2::ZERO, vec2(16.0, 16.0)))
        .finish();
    let handles = crate::gui::inspector::interactable_module::edit::compute_handles(
        Vec2::ZERO,
        ecs.get::<Interactable>(entity).unwrap(),
        16.0,
    );
    let handle = handles
        .iter()
        .find(|handle| handle.action == HandleAction::ResizeTop)
        .unwrap();
    let mouse_world = vec2(
        handle.rect.x + handle.rect.w * 0.5,
        handle.rect.y + handle.rect.h * 0.5,
    );

    let hit = selected_interactable_handle_hit(Some(entity), &ecs, mouse_world, 16.0);

    assert!(matches!(hit, Some((hit_entity, HandleAction::ResizeTop, _)) if hit_entity == entity));
}

#[test]
fn selected_interactable_edit_nudge_in_room_returns_updated_offset() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Interactable::circle(Vec2::ZERO, 20.0))
        .with_current_room(RoomId(7))
        .finish();

    let nudged = selected_interactable_edit_nudge(
        Some(entity),
        &ecs,
        RoomId(7),
        RoomLayer::Front,
        vec2(1.0, -1.0),
    );

    assert!(matches!(nudged, Some((nudged_entity, old_interactable, new_interactable))
        if nudged_entity == entity
        && old_interactable.offset == Vec2::ZERO
        && new_interactable.offset == vec2(1.0, -1.0)));
}

#[test]
fn resized_rect_interactable_bottom_right_drag_keeps_top_left_fixed() {
    let initial = Interactable::rect(Vec2::ZERO, vec2(10.0, 20.0));
    let transform = Transform {
        pivot: Pivot::TopLeft,
        ..Default::default()
    };
    let initial_rect = interactable_rect(&initial, transform);

    let resized = resized_rect_interactable(
        initial.clone(),
        transform,
        HandleAction::ResizeAabbBottomRight,
        vec2(6.0, 4.0),
    )
    .expect("rect interactable should resize");
    let resized_rect = interactable_rect(&resized, transform);

    assert_eq!(resized.rect_size, vec2(16.0, 24.0));
    assert_eq!(resized.offset, initial.offset + vec2(3.0, 2.0));
    assert_eq!(resized_rect.x, initial_rect.x);
    assert_eq!(resized_rect.y, initial_rect.y);
}

#[test]
fn resized_circle_interactable_drag_away_from_center_increases_radius() {
    let initial = Interactable::circle(Vec2::ZERO, 10.0);
    let transform = Transform::default();

    let resized = resized_circle_interactable(
        initial.clone(),
        transform,
        vec2(10.0, 0.0),
        vec2(14.0, 0.0),
    )
    .expect("circle interactable should resize");

    assert_eq!(resized.offset, initial.offset);
    assert_eq!(resized.radius, 14.0);
}
