use super::support::make_two_rooms;
use crate::engine::game_instance::GameInstance;
use crate::transitions::room_transition_manager::RoomTransitionManager;
use bishop::prelude::Vec2;
use engine_core::ecs::{Active, Collider, CurrentRoom, SubPixel, Transform};
use engine_core::game::Game;
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::Lua;
use std::collections::HashMap;

#[test]
fn handle_transitions_tracks_spatial_room_change() {
    let (room_a, room_b) = make_two_rooms();

    let mut world = World::default();
    world.add_room(room_a);
    world.add_room(room_b);
    world.grid_size = 1.0;
    world.rebuild_room_grid();

    let mut game = Game::default();
    game.add_world(world);
    let entity = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(40.0, 9.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(Active::default())
        .with_current_room(RoomId(1))
        .finish();

    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
    };

    RoomTransitionManager::handle_transitions(&Lua::new(), &mut game_instance);

    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| room.0),
        Some(RoomId(2))
    );
    let room_b_entities = game_instance.game.ecs.entities_in_room(RoomId(2));
    assert!(room_b_entities.contains(&entity));
    let room_a_entities = game_instance.game.ecs.entities_in_room(RoomId(1));
    assert!(!room_a_entities.contains(&entity));
}

#[test]
fn handle_transitions_ignores_entities_parked_in_inactive_worlds() {
    let (room_a, _room_b) = make_two_rooms();
    let mut active_world = World::new(WorldId(1), String::new(), 1.0);
    active_world.add_room(room_a);
    active_world.rebuild_room_grid();

    let mut inactive_world = World::new(WorldId(2), String::new(), 1.0);
    inactive_world.add_room(Room {
        id: RoomId(3),
        ..Default::default()
    });

    let mut game = Game::default();
    game.add_world(active_world);
    game.add_world(inactive_world);
    game.select_world(WorldId(1));

    let parked = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(5.0, 5.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(Active::default())
        .with_current_room(RoomId(3))
        .finish();

    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
    };

    RoomTransitionManager::handle_transitions(&Lua::new(), &mut game_instance);

    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(parked)
            .map(|room| room.0),
        Some(RoomId(3))
    );
}

#[test]
fn handle_transitions_uses_visual_position_instead_of_rounded_transform_position() {
    let (room_a, room_b) = make_two_rooms();

    let mut world = World::default();
    world.add_room(room_a);
    world.add_room(room_b);
    world.grid_size = 1.0;
    world.rebuild_room_grid();

    let mut game = Game::default();
    game.add_world(world);
    let entity = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(32.0, 9.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(Active::default())
        .with(SubPixel { x: -0.25, y: 0.0 })
        .with_current_room(RoomId(1))
        .finish();

    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
    };

    RoomTransitionManager::handle_transitions(&Lua::new(), &mut game_instance);

    assert_eq!(
        game_instance
            .game
            .ecs
            .get::<CurrentRoom>(entity)
            .map(|room| room.0),
        Some(RoomId(1))
    );
}
