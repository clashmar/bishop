use bishop::prelude::*;
use engine_core::assets::SpriteManager;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::physics_system::update_physics;

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
    ecs.create_entity()
        .with(Transform {
            position: Vec2::new(BOX_CENTER_X, FLOOR_Y),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: BOX_WIDTH,
                height: BOX_HEIGHT,
            },
            offset: Vec2::ZERO,
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

#[test]
fn physics_body_walking_into_same_floor_box_does_not_climb() {
    let sprite_manager = SpriteManager::default();
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

        update_physics(&sprite_manager, &mut ecs, &world, DT);

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
    let sprite_manager = SpriteManager::default();
    let room_id = RoomId(1);
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let player = spawn_player(&mut ecs, room_id);
    spawn_box(&mut ecs, room_id);

    for _ in 0..FRAMES {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = PLAYER_WALK_SPEED;
        }
        update_physics(&sprite_manager, &mut ecs, &world, DT);
    }

    let blocked_position = player_position(&ecs, player);

    for frame in 0..6 {
        if let Some(velocity) = ecs.get_mut::<Velocity>(player) {
            velocity.x = -PLAYER_WALK_SPEED;
        }
        update_physics(&sprite_manager, &mut ecs, &world, DT);

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
