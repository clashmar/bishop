use engine_core::ecs::ColliderShape;

#[test]
fn aabb_shape_returns_width_height_as_size() {
    let shape = ColliderShape::Aabb {
        width: 8.0,
        height: 12.0,
    };
    assert_eq!(shape.size(), (8.0, 12.0));
}

#[test]
fn circle_shape_returns_diameter_as_size() {
    let shape = ColliderShape::Circle { radius: 5.0 };
    assert_eq!(shape.size(), (10.0, 10.0));
}

#[test]
fn capsule_shape_returns_width_and_total_height() {
    let shape = ColliderShape::Capsule {
        radius: 4.0,
        height: 10.0,
    };
    assert_eq!(shape.size(), (8.0, 18.0));
}

#[test]
fn point_shape_returns_zero_size() {
    assert_eq!(ColliderShape::Point.size(), (0.0, 0.0));
}