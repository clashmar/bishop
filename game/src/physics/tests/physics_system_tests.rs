use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::physics_system::update_physics;

fn empty_room() -> Room {
    Room {
        id: RoomId(1),
        variants: vec![RoomVariant::default()],
        ..Default::default()
    }
}

fn empty_world() -> World {
    let room = empty_room();
    let mut world = World::default();
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn room_with_back_zones(room_id: RoomId, interior_zones: Vec<InteriorZone>) -> Room {
    Room {
        id: room_id,
        size: Vec2::new(4.0, 4.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(4, 4),
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    interior_zones,
                    ..Default::default()
                }),
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn motion_bodies_in_inactive_worlds_do_not_move() {
    let mut ecs = Ecs::default();
    let parked = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(10.0, 12.0),
            ..Default::default()
        })
        .with(Velocity { x: 120.0, y: 0.0 })
        .with(MotionBody)
        .with(SubPixel::default())
        .with_current_room(RoomId(99))
        .finish();

    update_physics(&mut ecs, &empty_world(), 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(parked)
            .map(|transform| transform.position),
        Some(Vec2::new(10.0, 12.0))
    );
}

#[test]
fn motion_bodies_roomed_in_the_active_world_still_move() {
    let mut ecs = Ecs::default();
    let roomed = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(10.0, 12.0),
            ..Default::default()
        })
        .with(Velocity { x: 120.0, y: 0.0 })
        .with(MotionBody)
        .with(SubPixel::default())
        .with_current_room(RoomId(1))
        .finish();

    update_physics(&mut ecs, &empty_world(), 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(roomed)
            .map(|transform| transform.position),
        Some(Vec2::new(12.0, 12.0))
    );
}

#[test]
fn velocity_entities_move_with_fixed_step_subpixel_accumulation() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(10.0, 12.0),
            ..Default::default()
        })
        .with(Velocity { x: 120.0, y: 0.0 })
        .with(SubPixel::default())
        .finish();

    update_physics(&mut ecs, &empty_world(), 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity)
            .map(|transform| transform.position),
        Some(Vec2::new(12.0, 12.0))
    );
    assert_eq!(
        ecs.get::<SubPixel>(entity)
            .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
        Some((0.0, 0.0))
    );
}

#[test]
fn motion_body_entities_move_with_fixed_step_subpixel_accumulation() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(10.0, 12.0),
            ..Default::default()
        })
        .with(Velocity { x: 120.0, y: 0.0 })
        .with(MotionBody)
        .with(SubPixel::default())
        .finish();

    update_physics(&mut ecs, &empty_world(), 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity)
            .map(|transform| transform.position),
        Some(Vec2::new(12.0, 12.0))
    );
    assert_eq!(
        ecs.get::<SubPixel>(entity)
            .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
        Some((0.0, 0.0))
    );
}

#[test]
fn physics_body_entities_accumulate_fractional_motion_in_subpixel() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Velocity { x: 30.0, y: 0.0 })
        .with(PhysicsBody)
        .with_current_room(RoomId(1))
        .with(Active::default())
        .with(SubPixel::default())
        .finish();

    update_physics(&mut ecs, &empty_world(), 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity)
            .map(|transform| transform.position),
        Some(Vec2::new(1.0, 0.0))
    );
    assert_eq!(
        ecs.get::<SubPixel>(entity)
            .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
        Some((-0.5, 0.0))
    );
}

#[test]
fn inactive_physics_body_is_not_simulated() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Velocity { x: 30.0, y: 0.0 })
        .with(Collider::default())
        .with(PhysicsBody)
        .with_current_room(RoomId(1))
        .with(Active::new(false))
        .finish();

    let world = empty_world();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position),
        Some(Vec2::ZERO)
    );
}

#[test]
fn physics_body_without_current_room_is_not_simulated() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Velocity { x: 30.0, y: 0.0 })
        .with(Collider::default())
        .with(PhysicsBody)
        .with(Active::default())
        .finish();

    let world = empty_world();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position),
        Some(Vec2::ZERO)
    );
}

#[test]
fn physics_body_uses_its_current_room_instead_of_world_current_room() {
    let room_a = Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: Vec2::new(32.0, 32.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(2, 2),
            ..Default::default()
        }],
        ..Default::default()
    };
    let room_b = Room {
        id: RoomId(2),
        position: Vec2::new(128.0, 0.0),
        size: Vec2::new(32.0, 32.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(2, 2),
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room_a.id);
    world.add_room(room_a);
    world.add_room(room_b);

    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(152.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with_current_room(RoomId(2))
        .with(Active::default())
        .with(PhysicsBody)
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(Velocity { x: 60.0, y: 0.0 })
        .finish();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position.x),
        Some(152.0)
    );
}

#[test]
fn grounded_capsule_stays_grounded_on_corner_support_with_subpixel_gap() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let entity = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(12.0, 14.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel { x: 0.0, y: -0.3 })
        .with_current_room(room_id)
        .with(Active::default())
        .finish();
    ecs.create_entity()
        .with(Transform {
            position: Vec2::new(0.0, 32.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 16.0,
                height: 16.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();

    let world = empty_world();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(ecs.get::<Grounded>(entity).map(|grounded| grounded.0), Some(true));
    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position),
        Some(Vec2::new(12.0, 14.0))
    );
}

#[test]
fn back_layer_physics_body_stays_within_effective_back_bounds_even_when_player_is_elsewhere() {
    let player_room = Room {
        id: RoomId(1),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(4, 4),
            ..Default::default()
        }],
        ..Default::default()
    };
    let npc_room = room_with_back_zones(
        RoomId(2),
        vec![InteriorZone {
            id: InteriorZoneId(1),
            bounds: InteriorZoneBounds::new(0, 0, 32, 64),
        }],
    );

    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(player_room.id);
    world.add_room(player_room);
    world.add_room(npc_room.clone());

    let mut ecs = Ecs::default();
    ecs.create_entity()
        .with(Player)
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();

    let npc = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(24.0, 24.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(PhysicsBody)
        .with(Velocity { x: 60.0, y: 0.0 })
        .with(Grounded(true))
        .with_current_room_layer(RoomId(2), RoomLayer::Back)
        .with(Active::default())
        .finish();

    ecs.create_entity()
        .with(Transform {
            position: Vec2::new(0.0, 32.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 32.0,
                height: 16.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .with_current_room_layer(RoomId(2), RoomLayer::Back)
        .finish();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(npc).map(|transform| transform.position),
        Some(Vec2::new(24.0, 24.0))
    );
}

#[test]
fn active_physics_body_is_simulated() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Velocity { x: 30.0, y: 0.0 })
        .with(Collider::default())
        .with(PhysicsBody)
        .with_current_room(RoomId(1))
        .with(Active::default())
        .finish();

    let world = empty_world();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position),
        Some(Vec2::new(1.0, 0.0))
    );
}

#[test]
fn pinned_inactive_physics_body_is_simulated() {
    let mut ecs = Ecs::default();
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Velocity { x: 30.0, y: 0.0 })
        .with(Collider::default())
        .with(PhysicsBody)
        .with_current_room(RoomId(1))
        .with(Active::new(false))
        .finish();
    ecs.get_mut::<Active>(entity).unwrap().pin();

    let world = empty_world();

    update_physics(&mut ecs, &world, 1.0 / 60.0);

    assert_eq!(
        ecs.get::<Transform>(entity).map(|transform| transform.position),
        Some(Vec2::new(1.0, 0.0))
    );
}