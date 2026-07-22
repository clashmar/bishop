use bishop::prelude::*;
use engine_core::tiles::TileRegistry;
use engine_core::ecs::*;
use engine_core::rendering::visual_position;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::physics_system::update_physics;
use crate::physics::shapes;

const DT: f32 = 1.0 / 60.0;
const GRID_SIZE: f32 = 16.0;
const ROOM_HEIGHT_TILES: usize = 4;
const FLOOR_Y: f32 = ROOM_HEIGHT_TILES as f32 * GRID_SIZE;
const PLAYER_WALK_SPEED: f32 = 100.0;
const PLAYER_RADIUS: f32 = 3.0;
const PLAYER_HEIGHT: f32 = 10.0;
const BOX_WIDTH: f32 = 8.0;
const BOX_HEIGHT: f32 = 8.0;
const BOX_CENTER_X: f32 = 44.0;
const BOX_LEFT_X: f32 = BOX_CENTER_X - BOX_WIDTH * 0.5;
const PLAYER_START_X: f32 = 28.0;
const MAX_BLOCKED_PLAYER_X: f32 = BOX_LEFT_X - PLAYER_RADIUS;
const FRAMES: usize = 24;
const DEMO_PLAYER_CIRCLE_RADIUS: f32 = 8.795493;
const DEMO_PLAYER_CIRCLE_OFFSET_X: f32 = 0.123633385;
const DEMO_PLAYER_CIRCLE_OFFSET_Y: f32 = 0.01086235;
const JUMP_SPEED: f32 = 200.0;

fn room_with_bottom_border() -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, ROOM_HEIGHT_TILES),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn world_with_bottom_border() -> World {
    let room = room_with_bottom_border();
    let mut world = World::default();
    world.grid_size = GRID_SIZE;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn world_with_bottom_exit_bridge() -> World {
    let mut room = room_with_bottom_border();
    room.exits = vec![
        Exit {
            position: Vec2::new(2.0, ROOM_HEIGHT_TILES as f32),
            direction: ExitDirection::Down,
            target_room_id: Some(RoomId(2)),
        },
        Exit {
            position: Vec2::new(4.0, ROOM_HEIGHT_TILES as f32),
            direction: ExitDirection::Down,
            target_room_id: Some(RoomId(3)),
        },
    ];

    let mut world = World::default();
    world.grid_size = GRID_SIZE;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn spawn_player(ecs: &mut Ecs, room_id: RoomId) -> Entity {
    ecs.create_entity()
        .with(Transform {
            position: Vec2::new(PLAYER_START_X, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish()
}

fn spawn_box(ecs: &mut Ecs, room_id: RoomId) {
    spawn_solid_aabb(
        ecs,
        room_id,
        Vec2::new(BOX_CENTER_X, FLOOR_Y),
        Pivot::BottomCenter,
        BOX_WIDTH,
        BOX_HEIGHT,
        Vec2::ZERO,
    );
}

fn spawn_solid_aabb(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    pivot: Pivot,
    width: f32,
    height: f32,
    offset: Vec2,
) {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width,
                height,
            },
            offset,
        })
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();
}

fn spawn_aabb_player(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish()
}

fn spawn_solid_circle(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    pivot: Pivot,
    radius: f32,
    offset: Vec2,
) {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius },
            offset,
        })
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();
}

fn player_position(ecs: &Ecs, player: Entity) -> Vec2 {
    match ecs.get::<Transform>(player) {
        Some(transform) => transform.position,
        None => panic!("player is missing Transform"),
    }
}

fn collider_bottom(ecs: &Ecs, player: Entity) -> f32 {
    let transform = ecs.get::<Transform>(player).copied().unwrap();
    let collider = ecs.get::<Collider>(player).copied().unwrap_or_default();
    let position = visual_position(transform.position, ecs.get::<SubPixel>(player));
    let (_, max) = shapes::collider_aabb(position, collider, transform.pivot);
    max.y
}

fn assert_capsule_jump_while_pressing_into_circle_matches_clear(
    start_x: f32,
    circle_x: f32,
    walk_velocity_x: f32,
) {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();

    let mut blocked_ecs = Ecs::default();
    let blocked_player = blocked_ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(start_x, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();
    spawn_solid_circle(
        &mut blocked_ecs,
        room_id,
        Vec2::new(circle_x, FLOOR_Y),
        Pivot::BottomCenter,
        8.0,
        Vec2::ZERO,
    );

    for _ in 0..6 {
        blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().x = walk_velocity_x;
        update_physics(&tile_registry, &mut blocked_ecs, &world, DT);
    }

    let blocked_transform = blocked_ecs.get::<Transform>(blocked_player).copied().unwrap();
    let blocked_sub_pixel = blocked_ecs
        .get::<SubPixel>(blocked_player)
        .copied()
        .unwrap();

    let mut clear_ecs = Ecs::default();
    let clear_player = clear_ecs
        .create_entity()
        .with(blocked_transform)
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(blocked_sub_pixel)
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    clear_ecs.get_mut::<Velocity>(clear_player).unwrap().x = walk_velocity_x;
    clear_ecs.get_mut::<Velocity>(clear_player).unwrap().y = -JUMP_SPEED;
    blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().x = walk_velocity_x;
    blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().y = -JUMP_SPEED;

    for frame in 0..6 {
        let blocked_pre_transform = blocked_ecs.get::<Transform>(blocked_player).copied().unwrap();
        let blocked_pre_sub_pixel = blocked_ecs.get::<SubPixel>(blocked_player).copied().unwrap();
        let blocked_pre_velocity = blocked_ecs.get::<Velocity>(blocked_player).copied().unwrap();

        update_physics(&tile_registry, &mut clear_ecs, &world, DT);
        update_physics(&tile_registry, &mut blocked_ecs, &world, DT);

        let clear_position = player_position(&clear_ecs, clear_player);
        let blocked_position = player_position(&blocked_ecs, blocked_player);
        let clear_velocity = clear_ecs.get::<Velocity>(clear_player).copied().unwrap();
        let blocked_velocity = blocked_ecs.get::<Velocity>(blocked_player).copied().unwrap();

        assert!(
            (blocked_position.y - clear_position.y).abs() < 0.01,
            "frame {frame}: capsule jump lost height while pressing into solid circle: clear={clear_position:?} blocked={blocked_position:?} clear_velocity={clear_velocity:?} blocked_velocity={blocked_velocity:?} pre_transform={blocked_pre_transform:?} pre_velocity={blocked_pre_velocity:?} pre_sub_pixel=({}, {}) walk_state={blocked_transform:?} sub_pixel=({}, {}) input_x={walk_velocity_x}",
            blocked_pre_sub_pixel.x,
            blocked_pre_sub_pixel.y,
            blocked_sub_pixel.x,
            blocked_sub_pixel.y
        );
        assert!(
            (blocked_velocity.y - clear_velocity.y).abs() < 0.01,
            "frame {frame}: capsule jump velocity diverged while pressing into solid circle: clear={clear_velocity:?} blocked={blocked_velocity:?} pre_transform={blocked_pre_transform:?} pre_velocity={blocked_pre_velocity:?} pre_sub_pixel=({}, {}) input_x={walk_velocity_x}",
            blocked_pre_sub_pixel.x,
            blocked_pre_sub_pixel.y
        );
    }

    assert_eq!(
        blocked_ecs.get::<Grounded>(blocked_player).map(|grounded| grounded.0),
        Some(false)
    );
}

#[test]
fn physics_body_walking_into_same_floor_box_does_not_climb() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = spawn_player(&mut ecs, room_id);
    spawn_box(&mut ecs, room_id);

    let mut positions = vec![player_position(&ecs, player)];

    for frame in 0..FRAMES {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }

        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        positions.push(position);

        assert!(
            position.y >= FLOOR_Y,
            "frame {frame}: player climbed onto box at {position:?}; path={positions:?}"
        );
        assert!(
            position.x <= MAX_BLOCKED_PLAYER_X,
            "frame {frame}: player advanced through box face to x={}; path={positions:?}",
            position.x
        );
    }

    let final_position = player_position(&ecs, player);
    assert!(
        final_position.x >= MAX_BLOCKED_PLAYER_X - 1.0,
        "player never reached the box face: final_position={final_position:?}; path={positions:?}"
    );
    assert_eq!(final_position.y, FLOOR_Y);
    assert_eq!(
        ecs.get::<Grounded>(player).map(|grounded| grounded.0),
        Some(true)
    );
}

#[test]
fn physics_body_can_walk_away_after_being_blocked_by_same_floor_box() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = spawn_player(&mut ecs, room_id);
    spawn_box(&mut ecs, room_id);

    for _ in 0..FRAMES {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);
    }

    let blocked_position = player_position(&ecs, player);

    for frame in 0..6 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = -PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        assert_eq!(
            position.y, FLOOR_Y,
            "frame {frame}: player left the floor while trying to back away: {position:?}"
        );
    }

    let released_position = player_position(&ecs, player);
    assert!(
        released_position.x < blocked_position.x,
        "player stayed stuck to the box: blocked_position={blocked_position:?} released_position={released_position:?}"
    );
}

#[test]
fn physics_body_can_walk_right_away_after_being_blocked_from_right_by_same_floor_box() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let start_x = BOX_CENTER_X + 16.0;
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(start_x, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();
    spawn_box(&mut ecs, room_id);

    for _ in 0..FRAMES {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = -PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);
    }

    let blocked_position = player_position(&ecs, player);

    for frame in 0..6 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        assert_eq!(
            position.y, FLOOR_Y,
            "frame {frame}: player left the floor while trying to back away rightward: {position:?}"
        );
    }

    let released_position = player_position(&ecs, player);
    assert!(
        released_position.x > blocked_position.x,
        "player stayed stuck to the box after left-side block: blocked_position={blocked_position:?} released_position={released_position:?}"
    );
}

#[test]
fn physics_body_can_walk_right_away_after_being_blocked_from_right_by_tall_box() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(60.0, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();
    spawn_solid_aabb(
        &mut ecs,
        room_id,
        Vec2::new(44.0, FLOOR_Y),
        Pivot::BottomCenter,
        8.0,
        20.0,
        Vec2::ZERO,
    );

    for _ in 0..FRAMES {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = -PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);
    }

    let blocked_position = player_position(&ecs, player);

    for frame in 0..6 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        assert_eq!(
            position.y, blocked_position.y,
            "frame {frame}: player vertical position changed while releasing from tall box: {position:?} start={blocked_position:?}"
        );
    }

    let released_position = player_position(&ecs, player);
    assert!(
        released_position.x > blocked_position.x,
        "player stayed stuck to tall box after left-side block: blocked_position={blocked_position:?} released_position={released_position:?}"
    );
}

#[test]
fn physics_body_aabb_moves_horizontally_on_flat_floor_with_solid_circle_in_room() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = spawn_aabb_player(&mut ecs, room_id, Vec2::new(PLAYER_START_X, FLOOR_Y));
    spawn_solid_circle(
        &mut ecs,
        room_id,
        Vec2::new(112.0, FLOOR_Y),
        Pivot::BottomCenter,
        8.0,
        Vec2::ZERO,
    );

    let start_position = player_position(&ecs, player);

    for frame in 0..12 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        assert!(
            position.y <= FLOOR_Y,
            "frame {frame}: aabb player sank below floor while walking: {position:?}"
        );
    }

    let final_position = player_position(&ecs, player);
    assert!(
        final_position.x > start_position.x + 8.0,
        "aabb player stuck on flat floor: start={start_position:?} final={final_position:?}"
    );
}

#[test]
fn physics_body_aabb_can_jump_beside_solid_circle() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = spawn_aabb_player(&mut ecs, room_id, Vec2::new(40.0, FLOOR_Y));
    spawn_solid_circle(
        &mut ecs,
        room_id,
        Vec2::new(52.0, FLOOR_Y),
        Pivot::BottomCenter,
        8.0,
        Vec2::ZERO,
    );

    let start_position = player_position(&ecs, player);
    if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
        velocity.y = -JUMP_SPEED;
    }

    update_physics(&tile_registry, &mut ecs, &world, DT);

    let final_position = player_position(&ecs, player);
    let final_velocity = ecs.get::<Velocity>(player).copied().unwrap();
    assert!(
        final_position.y < start_position.y - 1.0,
        "aabb player failed to jump beside solid circle: start={start_position:?} final={final_position:?} velocity={final_velocity:?}"
    );
    assert!(
        final_velocity.y < 0.0,
        "aabb jump velocity was cancelled beside solid circle: {final_velocity:?}"
    );
    assert_eq!(ecs.get::<Grounded>(player).map(|grounded| grounded.0), Some(false));
}

#[test]
fn physics_body_capsule_can_jump_beside_solid_circle() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();

    let mut blocked_ecs = Ecs::default();
    let blocked_player = blocked_ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(40.0, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();
    spawn_solid_circle(
        &mut blocked_ecs,
        room_id,
        Vec2::new(52.0, FLOOR_Y),
        Pivot::BottomCenter,
        8.0,
        Vec2::ZERO,
    );

    for _ in 0..6 {
        blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().x = PLAYER_WALK_SPEED;
        update_physics(&tile_registry, &mut blocked_ecs, &world, DT);
    }

    let blocked_transform = blocked_ecs.get::<Transform>(blocked_player).copied().unwrap();
    let blocked_sub_pixel = blocked_ecs
        .get::<SubPixel>(blocked_player)
        .copied()
        .unwrap();

    let mut clear_ecs = Ecs::default();
    let clear_player = clear_ecs
        .create_entity()
        .with(blocked_transform)
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(blocked_sub_pixel)
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    clear_ecs.get_mut::<Velocity>(clear_player).unwrap().y = -JUMP_SPEED;
    blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().x = 0.0;
    blocked_ecs.get_mut::<Velocity>(blocked_player).unwrap().y = -JUMP_SPEED;

    for frame in 0..6 {
        let blocked_pre_transform = blocked_ecs.get::<Transform>(blocked_player).copied().unwrap();
        let blocked_pre_sub_pixel = blocked_ecs.get::<SubPixel>(blocked_player).copied().unwrap();
        let blocked_pre_velocity = blocked_ecs.get::<Velocity>(blocked_player).copied().unwrap();

        update_physics(&tile_registry, &mut clear_ecs, &world, DT);
        update_physics(&tile_registry, &mut blocked_ecs, &world, DT);

        let clear_position = player_position(&clear_ecs, clear_player);
        let blocked_position = player_position(&blocked_ecs, blocked_player);
        let clear_velocity = clear_ecs.get::<Velocity>(clear_player).copied().unwrap();
        let blocked_velocity = blocked_ecs.get::<Velocity>(blocked_player).copied().unwrap();

        assert!(
            (blocked_position.y - clear_position.y).abs() < 0.01,
            "frame {frame}: capsule jump lost height beside solid circle: clear={clear_position:?} blocked={blocked_position:?} clear_velocity={clear_velocity:?} blocked_velocity={blocked_velocity:?} pre_transform={blocked_pre_transform:?} pre_velocity={blocked_pre_velocity:?} pre_sub_pixel=({}, {}) walk_state={blocked_transform:?} sub_pixel=({}, {})",
            blocked_pre_sub_pixel.x,
            blocked_pre_sub_pixel.y,
            blocked_sub_pixel.x,
            blocked_sub_pixel.y
        );
        assert!(
            (blocked_velocity.y - clear_velocity.y).abs() < 0.01,
            "frame {frame}: capsule jump velocity diverged beside solid circle: clear={clear_velocity:?} blocked={blocked_velocity:?} pre_transform={blocked_pre_transform:?} pre_velocity={blocked_pre_velocity:?} pre_sub_pixel=({}, {})",
            blocked_pre_sub_pixel.x,
            blocked_pre_sub_pixel.y
        );
    }
    assert_eq!(blocked_ecs.get::<Grounded>(blocked_player).map(|grounded| grounded.0), Some(false));
}

#[test]
fn physics_body_capsule_can_jump_while_pressing_right_into_solid_circle() {
    assert_capsule_jump_while_pressing_into_circle_matches_clear(
        40.0,
        52.0,
        PLAYER_WALK_SPEED,
    );
}

#[test]
fn physics_body_capsule_can_jump_while_pressing_left_into_solid_circle() {
    assert_capsule_jump_while_pressing_into_circle_matches_clear(
        64.0,
        52.0,
        -PLAYER_WALK_SPEED,
    );
}

#[test]
fn physics_body_capsule_stays_on_floor_between_bottom_exits() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_exit_bridge();
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(50.0, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: PLAYER_RADIUS,
                height: PLAYER_HEIGHT,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    for frame in 0..8 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        assert!(
            position.y <= FLOOR_Y,
            "frame {frame}: capsule fell through the bridge floor between exits: {position:?}"
        );
    }
}

#[test]
fn physics_body_demo_circle_resting_on_flat_floor_keeps_exact_contact_height() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(PLAYER_START_X, FLOOR_Y - DEMO_PLAYER_CIRCLE_OFFSET_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Circle {
                radius: DEMO_PLAYER_CIRCLE_RADIUS,
            },
            offset: Vec2::new(DEMO_PLAYER_CIRCLE_OFFSET_X, DEMO_PLAYER_CIRCLE_OFFSET_Y),
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    update_physics(&tile_registry, &mut ecs, &world, DT);

    let bottom = collider_bottom(&ecs, player);
    assert!(
        (bottom - FLOOR_Y).abs() < 0.001,
        "demo circle resting height drifted into floor: bottom={bottom} floor={FLOOR_Y} position={:?}",
        player_position(&ecs, player)
    );
}

#[test]
fn physics_body_demo_circle_moves_horizontally_on_flat_floor() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(PLAYER_START_X, FLOOR_Y - DEMO_PLAYER_CIRCLE_OFFSET_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Circle {
                radius: DEMO_PLAYER_CIRCLE_RADIUS,
            },
            offset: Vec2::new(DEMO_PLAYER_CIRCLE_OFFSET_X, DEMO_PLAYER_CIRCLE_OFFSET_Y),
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    let start_position = player_position(&ecs, player);

    for frame in 0..12 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        let bottom = collider_bottom(&ecs, player);
        assert!(
            bottom <= FLOOR_Y + 0.001,
            "frame {frame}: demo circle player sank below floor while walking: position={position:?} bottom={bottom}"
        );
    }

    let final_position = player_position(&ecs, player);
    assert!(
        final_position.x > start_position.x + 8.0,
        "demo circle player stuck on flat floor: start={start_position:?} final={final_position:?}"
    );
}

#[test]
fn physics_body_demo_circle_moves_left_on_flat_floor() {
    let tile_registry = TileRegistry::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let start_x = PLAYER_START_X + 40.0;
    let player = ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(start_x, FLOOR_Y - DEMO_PLAYER_CIRCLE_OFFSET_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Circle {
                radius: DEMO_PLAYER_CIRCLE_RADIUS,
            },
            offset: Vec2::new(DEMO_PLAYER_CIRCLE_OFFSET_X, DEMO_PLAYER_CIRCLE_OFFSET_Y),
        })
        .with(PhysicsBody)
        .with(Grounded(true))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish();

    let start_position = player_position(&ecs, player);

    for frame in 0..12 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = -PLAYER_WALK_SPEED;
        }
        update_physics(&tile_registry, &mut ecs, &world, DT);

        let position = player_position(&ecs, player);
        let bottom = collider_bottom(&ecs, player);
        assert!(
            bottom <= FLOOR_Y + 0.001,
            "frame {frame}: demo circle player sank below floor while moving left: position={position:?} bottom={bottom}"
        );
    }

    let final_position = player_position(&ecs, player);
    assert!(
        final_position.x < start_position.x - 8.0,
        "demo circle player stuck moving left on flat floor: start={start_position:?} final={final_position:?}"
    );
}
