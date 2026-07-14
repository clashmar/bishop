use bishop::prelude::*;
use engine_core::ecs::*;

use crate::physics::shapes::*;

#[test]
fn collider_aabb_aabb_top_left_pivot_no_offset() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Aabb {
            width: 8.0,
            height: 12.0,
        },
        offset: Vec2::ZERO,
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::TopLeft);
    assert_eq!(min, Vec2::new(10.0, 20.0));
    assert_eq!(max, Vec2::new(18.0, 32.0));
}

#[test]
fn collider_aabb_aabb_center_pivot_no_offset() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Aabb {
            width: 8.0,
            height: 12.0,
        },
        offset: Vec2::ZERO,
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::Center);
    assert_eq!(min, Vec2::new(6.0, 14.0));
    assert_eq!(max, Vec2::new(14.0, 26.0));
}

#[test]
fn collider_aabb_aabb_with_offset() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Aabb {
            width: 8.0,
            height: 8.0,
        },
        offset: Vec2::new(3.0, -2.0),
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::TopLeft);
    assert_eq!(min, Vec2::new(13.0, 18.0));
    assert_eq!(max, Vec2::new(21.0, 26.0));
}

#[test]
fn collider_aabb_circle_top_left_pivot() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Circle { radius: 5.0 },
        offset: Vec2::ZERO,
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::TopLeft);
    assert_eq!(min, Vec2::new(10.0, 20.0));
    assert_eq!(max, Vec2::new(20.0, 30.0));
}

#[test]
fn collider_aabb_capsule_top_left_pivot() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Capsule {
            radius: 4.0,
            height: 10.0,
        },
        offset: Vec2::ZERO,
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::TopLeft);
    assert_eq!(min, Vec2::new(10.0, 20.0));
    assert_eq!(max, Vec2::new(18.0, 38.0));
}

#[test]
fn collider_aabb_point_top_left_pivot() {
    let pos = Vec2::new(10.0, 20.0);
    let collider = Collider {
        shape: ColliderShape::Point,
        offset: Vec2::ZERO,
    };
    let (min, max) = collider_aabb(pos, collider, Pivot::TopLeft);
    assert_eq!(min, Vec2::new(10.0, 20.0));
    assert_eq!(max, Vec2::new(10.0, 20.0));
}

#[test]
fn sweep_axis_aabb_x_positive_blocked() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    // Moving right by 20, blocked by obstacle at x=10
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, Some(2.0)); // can move 2.0 before hitting obstacle at x=10
}

#[test]
fn sweep_axis_aabb_x_negative_blocked() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(20.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    // Moving left by -20, blocked by obstacle ending at x=18
    let result = sweep_axis(shape, shape_pos, -20.0, 0, obstacle);
    assert_eq!(result, Some(-2.0)); // can move -2.0 before hitting obstacle at x=18
}

#[test]
fn sweep_axis_aabb_x_no_overlap_on_perpendicular_axis() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(0.0, 100.0); // far away on Y
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, None); // no collision — shapes don't overlap on Y
}

#[test]
fn sweep_axis_aabb_x_no_block_when_not_reaching_obstacle() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    // Moving right by 1.0 — doesn't reach obstacle
    let result = sweep_axis(shape, shape_pos, 1.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_aabb_y_positive_blocked() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(0.0, 10.0), Vec2::new(8.0, 18.0));
    // Moving down by 20, blocked by obstacle at y=10
    let result = sweep_axis(shape, shape_pos, 20.0, 1, obstacle);
    assert_eq!(result, Some(2.0));
}

#[test]
fn sweep_axis_aabb_y_negative_blocked() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 8.0,
    };
    let shape_pos = Vec2::new(0.0, 20.0);
    let obstacle = (Vec2::new(0.0, 10.0), Vec2::new(8.0, 18.0));
    // Moving up by -20, blocked by obstacle ending at y=18
    let result = sweep_axis(shape, shape_pos, -20.0, 1, obstacle);
    assert_eq!(result, Some(-2.0));
}

#[test]
fn sweep_axis_circle_x_positive_blocked() {
    let shape = ColliderShape::Circle { radius: 5.0 };
    // Circle center at (5, 5), bounding AABB top-left at (0, 0)
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(12.0, 0.0), Vec2::new(20.0, 10.0));
    // Circle right edge at x=10, obstacle at x=12.
    // Allowed = 12 - 10 = 2.0
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, Some(2.0));
}

#[test]
fn sweep_axis_circle_x_negative_blocked() {
    let shape = ColliderShape::Circle { radius: 5.0 };
    let shape_pos = Vec2::new(20.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 10.0));
    // Circle center at (25, 5), moving left. Obstacle right edge at x=18.
    // Allowed = 18 - 20 = -2.0
    let result = sweep_axis(shape, shape_pos, -20.0, 0, obstacle);
    assert_eq!(result, Some(-2.0));
}

#[test]
fn sweep_axis_circle_x_no_overlap_on_perpendicular() {
    let shape = ColliderShape::Circle { radius: 5.0 };
    let shape_pos = Vec2::new(0.0, 100.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_circle_x_grazing_passes_through() {
    let shape = ColliderShape::Circle { radius: 5.0 };
    // Circle center at (5, 5), bounding AABB top-left at (0, 0)
    let shape_pos = Vec2::new(0.0, 0.0);
    // Obstacle far enough that circle doesn't reach it
    let obstacle = (Vec2::new(20.0, 0.0), Vec2::new(28.0, 10.0));
    let result = sweep_axis(shape, shape_pos, 10.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_circle_x_passes_below_corner() {
    // Circle AABB overlaps obstacle on Y, but the circle's center is below
    // the obstacle — the circle should pass under, not get blocked.
    let shape = ColliderShape::Circle { radius: 5.0 };
    let shape_pos = Vec2::new(0.0, 1.0);
    let obstacle = (Vec2::new(10.0, 10.0), Vec2::new(18.0, 18.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_capsule_x_positive_blocked() {
    let shape = ColliderShape::Capsule {
        radius: 4.0,
        height: 10.0,
    };
    // Capsule AABB: width=8, height=18. Top-left at (0, 0).
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 18.0));
    // Capsule right edge at x=8, obstacle at x=10.
    // Allowed = 10 - 8 = 2.0
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, Some(2.0));
}

#[test]
fn sweep_axis_capsule_x_negative_blocked() {
    let shape = ColliderShape::Capsule {
        radius: 4.0,
        height: 10.0,
    };
    let shape_pos = Vec2::new(20.0, 0.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 18.0));
    // Capsule left edge at x=20, obstacle right edge at x=18.
    // Allowed = 18 - 20 = -2.0
    let result = sweep_axis(shape, shape_pos, -20.0, 0, obstacle);
    assert_eq!(result, Some(-2.0));
}

#[test]
fn sweep_axis_capsule_x_no_overlap_on_perpendicular() {
    let shape = ColliderShape::Capsule {
        radius: 4.0,
        height: 10.0,
    };
    let shape_pos = Vec2::new(0.0, 100.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_capsule_y_positive_blocked() {
    let shape = ColliderShape::Capsule {
        radius: 4.0,
        height: 10.0,
    };
    let shape_pos = Vec2::new(0.0, 0.0);
    let obstacle = (Vec2::new(0.0, 20.0), Vec2::new(8.0, 28.0));
    // Capsule bottom at y=18, obstacle at y=20.
    // Allowed = 20 - 18 = 2.0
    let result = sweep_axis(shape, shape_pos, 20.0, 1, obstacle);
    assert_eq!(result, Some(2.0));
}

#[test]
fn sweep_axis_point_x_positive_blocked() {
    let shape = ColliderShape::Point;
    let shape_pos = Vec2::new(0.0, 5.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 10.0));
    // Point at x=0, obstacle at x=10. Allowed = 10.0
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, Some(10.0));
}

#[test]
fn sweep_axis_point_x_negative_blocked() {
    let shape = ColliderShape::Point;
    let shape_pos = Vec2::new(20.0, 5.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 10.0));
    // Point at x=20, obstacle right edge at x=18. Allowed = -2.0
    let result = sweep_axis(shape, shape_pos, -20.0, 0, obstacle);
    assert_eq!(result, Some(-2.0));
}

#[test]
fn sweep_axis_point_x_no_overlap_on_perpendicular() {
    let shape = ColliderShape::Point;
    let shape_pos = Vec2::new(0.0, 100.0);
    let obstacle = (Vec2::new(10.0, 0.0), Vec2::new(18.0, 8.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 0, obstacle);
    assert_eq!(result, None);
}

#[test]
fn sweep_axis_point_y_positive_blocked() {
    let shape = ColliderShape::Point;
    let shape_pos = Vec2::new(5.0, 0.0);
    let obstacle = (Vec2::new(0.0, 10.0), Vec2::new(10.0, 18.0));
    let result = sweep_axis(shape, shape_pos, 20.0, 1, obstacle);
    assert_eq!(result, Some(10.0));
}