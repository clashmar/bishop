use crate::ecs::*;
use crate::game::Game;
use crate::worlds::test_utils::make_room;
use crate::worlds::*;

fn game_with_two_worlds() -> Game {
    let mut game = Game::default();
    for id in 1..=2 {
        game.add_world(World::from_rooms(
            WorldId(id),
            String::new(),
            vec![make_room(Some(id), 0.0, 0.0, 4.0, 4.0)],
            16.0,
        ));
    }
    game.select_world(WorldId(1));
    game
}

#[test]
fn entity_in_active_world_accepts_entity_roomed_in_current_world() {
    let mut game = game_with_two_worlds();
    let entity = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();

    assert!(game.entity_in_active_world(entity));
}

#[test]
fn entity_in_active_world_rejects_entity_roomed_in_inactive_world() {
    let mut game = game_with_two_worlds();
    let entity = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(2))
        .finish();

    assert!(!game.entity_in_active_world(entity));
}

#[test]
fn entity_in_active_world_accepts_entity_without_current_room() {
    let mut game = game_with_two_worlds();
    let entity = game.ecs.create_entity().finish();

    assert!(game.entity_in_active_world(entity));
}
