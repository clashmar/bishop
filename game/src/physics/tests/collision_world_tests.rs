use super::*;

fn world_with_solid(aabb: (Vec2, Vec2)) -> CollisionWorld {
    let shape = ColliderShape::Aabb {
        width: aabb.1.x - aabb.0.x,
        height: aabb.1.y - aabb.0.y,
    };
    CollisionWorld {
        solids: vec![SolidObj {
            aabb,
            shape,
            shape_pos: aabb.0,
            entity: None,
        }],
    }
}

fn dummy_entity() -> Entity {
    Entity::null()
}

#[test]
fn circle_cast_corner_misses_when_too_far() {
    let result = circle_cast_corner(
        Vec2::new(0.0, -10.0),
        Vec2::new(10.0, 0.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    assert!(result.is_none());
}

#[test]
fn circle_cast_corner_entry_in_past_when_already_inside() {
    let result = circle_cast_corner(
        Vec2::new(0.0, -7.0),
        Vec2::new(10.0, 0.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    assert!(result.is_none());
}

#[test]
fn circle_cast_corner_hits_diagonal_approach() {
    let result = circle_cast_corner(
        Vec2::new(-10.0, -10.0),
        Vec2::new(10.0, 10.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    let t = match result {
        Some(t) => t,
        None => panic!("circle should hit the corner"),
    };
    assert!(t > 0.4 && t < 0.5, "expected t ≈ 0.434, got {t}");
}

#[test]
fn sweep_circle_blocked_by_corner_from_above() {
    let world = world_with_solid((
        Vec2::new(0.0, 0.0),
        Vec2::new(16.0, 16.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(-10.0, -7.0),
        8.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
    );
    assert!(result.blocked_x, "circle should be blocked horizontally by corner");
    assert!(result.t_x < 1.0, "t_x should be less than 1.0");
}

#[test]
fn sweep_circle_not_blocked_when_passing_above_obstacle() {
    let world = world_with_solid((
        Vec2::new(0.0, 0.0),
        Vec2::new(16.0, 16.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(0.0, -17.0),
        8.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
    );
    assert!(!result.blocked_x, "circle should pass above obstacle");
    assert!(!result.blocked_y, "circle should pass above obstacle");
}

#[test]
fn sweep_capsule_not_pushed_up_when_walking_into_wall() {
    let world = world_with_solid((
        Vec2::new(16.0, -8.0),
        Vec2::new(24.0, 0.0),
    ));
    let result = world.sweep_capsule(
        Vec2::new(0.0, -17.0),
        8.0,
        16.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
    );
    assert!(result.blocked_x, "capsule should be blocked horizontally");
    assert!(
        result.push_y.abs() < 0.01,
        "capsule should not be pushed up, got push_y={}",
        result.push_y
    );
}

#[test]
fn circle_depenetration_pushes_along_dominant_axis_only() {
    let world = world_with_solid((
        Vec2::new(16.0, -8.0),
        Vec2::new(24.0, 0.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(10.0, -8.5),
        8.0,
        Vec2::new(2.0, 0.0),
        dummy_entity(),
    );
    assert!(
        result.push_x < -0.01,
        "circle should be pushed left, got push_x={}",
        result.push_x
    );
    assert!(
        result.push_y.abs() < 0.01,
        "circle should not be pushed up, got push_y={}",
        result.push_y
    );
}

#[test]
fn capsule_walking_into_wall_multi_frame_no_climb() {
    let world = CollisionWorld {
        solids: vec![
            SolidObj {
                aabb: (Vec2::new(16.0, -8.0), Vec2::new(24.0, 0.0)),
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                shape_pos: Vec2::new(16.0, -8.0),
                entity: None,
            },
            SolidObj {
                aabb: (Vec2::new(0.0, 0.0), Vec2::new(32.0, 16.0)),
                shape: ColliderShape::Aabb {
                    width: 32.0,
                    height: 16.0,
                },
                shape_pos: Vec2::new(0.0, 0.0),
                entity: None,
            },
        ],
    };
    let mut center = Vec2::new(0.0, -16.0);
    let radius = 8.0;
    let height = 16.0;
    let dt = 1.0 / 60.0;
    let gravity = 800.0;
    let walk_speed = 120.0;
    let mut vel_y = 0.0f32;
    let mut sub_pixel = SubPixel::default();
    let entity = dummy_entity();

    for _frame in 0..120 {
        vel_y += gravity * dt;
        let delta = Vec2::new(walk_speed * dt, vel_y * dt);
        let true_pos = center + Vec2::new(sub_pixel.x, sub_pixel.y);

        let sweep = world.sweep_capsule(true_pos, radius, height, delta, entity);
        let result = sweep.finish(delta);

        let new_true = true_pos + result.allowed_delta;
        let new_int = new_true.round();
        sub_pixel.x = new_true.x - new_int.x;
        sub_pixel.y = new_true.y - new_int.y;
        center = new_int;

        if result.blocked_x {
            sub_pixel.x = 0.0;
        }
        if result.blocked_y {
            vel_y = 0.0;
            sub_pixel.y = 0.0;
        }

        assert!(
            center.y >= -16.0,
            "frame {_frame}: capsule climbed to y={}, started at y=-16",
            center.y
        );
        let body_right = center.x + radius;
        assert!(
            body_right <= 16.0 + 0.01,
            "frame {_frame}: capsule body right edge at {body_right}, obstacle left at 16"
        );
    }
}
