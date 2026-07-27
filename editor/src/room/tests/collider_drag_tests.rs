use bishop::prelude::{vec2, Rect, Vec2};
use engine_core::ecs::*;
use engine_core::worlds::{RoomId, RoomLayer};

use super::{
    resized_aabb_collider,
    resized_aabb_collider_uniform,
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
        16.0,
    );
    let mouse_world = match handles.last() {
        Some(handle) => vec2(
            handle.rect.x + handle.rect.w * 0.5,
            handle.rect.y + handle.rect.h * 0.5,
        ),
        None => panic!("expected collider handles for default collider"),
    };

    let hit = selected_collider_handle_hit(Some(entity), &ecs, mouse_world, 16.0);

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

    let hit = selected_collider_handle_hit(Some(entity), &ecs, Vec2::ZERO, 16.0);

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

    let nudged = selected_collider_edit_nudge(
        Some(entity),
        &ecs,
        RoomId(7),
        RoomLayer::Front,
        vec2(1.0, -1.0),
    );

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
fn selected_collider_edit_nudge_other_layer_returns_none() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Collider::default())
        .with_current_room_layer(RoomId(7), RoomLayer::Back)
        .finish();

    let nudged = selected_collider_edit_nudge(
        Some(entity),
        &ecs,
        RoomId(7),
        RoomLayer::Front,
        vec2(1.0, -1.0),
    );

    assert!(nudged.is_none());
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

#[test]
fn resized_aabb_collider_top_edge_drag_resizes_height_top_anchored() {
    let initial = Collider {
        shape: ColliderShape::Aabb { width: 10.0, height: 20.0 },
        ..Default::default()
    };
    let transform = Transform { pivot: Pivot::TopLeft, ..Default::default() };
    let initial_rect = aabb_rect(initial, transform);

    let resized = resized_aabb_collider(
        initial,
        transform,
        crate::gui::inspector::collider_module::edit::HandleAction::ResizeTop,
        vec2(0.0, -5.0),
    );

    match resized {
        Some(collider) => {
            let resized_rect = aabb_rect(collider, transform);
            // Bottom edge stays fixed
            assert_eq!(resized_rect.y + resized_rect.h, initial_rect.y + initial_rect.h);
            assert_eq!(resized_rect.w, 10.0); // width unchanged
            assert_eq!(resized_rect.h, 25.0);
        }
        None => panic!("expected resized collider"),
    }
}

#[test]
fn resized_aabb_collider_left_edge_drag_resizes_width_left_anchored() {
    let initial = Collider {
        shape: ColliderShape::Aabb { width: 10.0, height: 20.0 },
        ..Default::default()
    };
    let transform = Transform { pivot: Pivot::TopLeft, ..Default::default() };
    let initial_rect = aabb_rect(initial, transform);

    let resized = resized_aabb_collider(
        initial,
        transform,
        crate::gui::inspector::collider_module::edit::HandleAction::ResizeLeft,
        vec2(-3.0, 0.0),
    );

    match resized {
        Some(collider) => {
            let resized_rect = aabb_rect(collider, transform);
            // Right edge stays fixed
            assert_eq!(resized_rect.x + resized_rect.w, initial_rect.x + initial_rect.w);
            assert_eq!(resized_rect.h, 20.0); // height unchanged
            assert_eq!(resized_rect.w, 13.0);
        }
        None => panic!("expected resized collider"),
    }
}

#[test]
fn resized_aabb_collider_uniform_resize_keeps_center_fixed_and_makes_square() {
    let initial = Collider {
        shape: ColliderShape::Aabb { width: 10.0, height: 20.0 },
        ..Default::default()
    };
    let transform = Transform { pivot: Pivot::TopLeft, ..Default::default() };
    let initial_rect = aabb_rect(initial, transform);
    let initial_center_x = initial_rect.x + initial_rect.w / 2.0;
    let initial_center_y = initial_rect.y + initial_rect.h / 2.0;

    let resized = resized_aabb_collider_uniform(
        initial,
        transform,
        crate::gui::inspector::collider_module::edit::HandleAction::ResizeAabbTopRight,
        vec2(4.0, -4.0),
    );

    match resized {
        Some(collider) => {
            let resized_rect = aabb_rect(collider, transform);
            let new_center_x = resized_rect.x + resized_rect.w / 2.0;
            let new_center_y = resized_rect.y + resized_rect.h / 2.0;
            // Center stays fixed
            assert!((new_center_x - initial_center_x).abs() < 0.01);
            assert!((new_center_y - initial_center_y).abs() < 0.01);
            // Both dimensions change equally (square)
            assert!((resized_rect.w - resized_rect.h).abs() < 0.01);
        }
        None => panic!("expected resized collider"),
    }
}
