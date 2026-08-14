use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::worlds::RoomId;

use crate::physics::collision_world::*;

use super::super::test_support::empty_world;
use super::test_support::{
    assert_capsule_pressing_into_circle_jump_keeps_vertical_motion,
    world_with_bottom_border,
};
use super::*;
use super::super::SolidObj;
use super::sweep_math::circle_cast_corner;

mod aabb_sweep_tests;
mod capsule_circle_sweep_tests;
mod capsule_tile_sweep_tests;
mod circle_sweep_tests;

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
            layer: None,
            interior_zone: None,
        }],
        entity_layers: Default::default(),
        back_interior_zones: Vec::new(),
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
        None => panic!("expected diagonal corner hit"),
    };
    assert!((t - 0.434_314_58).abs() < 1e-5, "t={t}");
}

#[test]
fn sweep_circle_not_blocked_when_passing_above_obstacle() {
    let cw = world_with_solid((Vec2::new(0.0, 0.0), Vec2::new(16.0, 16.0)));
    let res = cw.sweep_circle(
        Vec2::new(-8.0, -17.0),
        8.0,
        Vec2::new(40.0, 0.0),
        SweepContext {
            moving_entity: dummy_entity(),
            moving_layer: RoomLayer::Front,
            active_back_zone: None,
        },
    );
    assert!(!res.blocked_x);
    assert!(!res.blocked_y);
    assert_eq!(res.push_x, 0.0);
    assert_eq!(res.push_y, 0.0);
}

