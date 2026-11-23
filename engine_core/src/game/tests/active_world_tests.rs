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

#[test]
fn world_of_room_returns_owning_world() {
    let game = game_with_two_worlds();

    assert_eq!(game.world_of_room(RoomId(2)).map(|w| w.id), Some(WorldId(2)));
    assert!(game.world_of_room(RoomId(99)).is_none());
}

#[cfg(feature = "editor")]
#[test]
fn ctx_mut_for_world_targets_requested_world_without_changing_current_world() {
    let mut game = game_with_two_worlds();
    let current_world_id = game.current_world_id;

    {
        let game_ctx = game.ctx_mut_for_world(WorldId(2));
        assert_eq!(
            game_ctx.world.as_ref().map(|world| world.id),
            Some(WorldId(2))
        );
        assert_eq!(
            game_ctx
                .world_directory
                .iter()
                .map(|world| world.id)
                .collect::<Vec<_>>(),
            vec![WorldId(1), WorldId(2)]
        );
        assert_eq!(game_ctx.room_world_map.get(&RoomId(1)), Some(&WorldId(1)));
        assert_eq!(game_ctx.room_world_map.get(&RoomId(2)), Some(&WorldId(2)));
    }

    assert_eq!(game.current_world_id, current_world_id);
}

#[test]
fn ctx_mut_targets_current_world_and_exposes_game_wide_world_data() {
    let mut game = game_with_two_worlds();
    let current_world_id = game.current_world_id;

    let game_ctx = game.ctx_mut();

    assert_eq!(
        game_ctx.world.as_ref().map(|world| world.id),
        current_world_id
    );
    assert_eq!(
        game_ctx
            .world_directory
            .iter()
            .map(|world| world.id)
            .collect::<Vec<_>>(),
        vec![WorldId(1), WorldId(2)]
    );
    assert_eq!(game_ctx.room_world_map.get(&RoomId(1)), Some(&WorldId(1)));
    assert_eq!(game_ctx.room_world_map.get(&RoomId(2)), Some(&WorldId(2)));
}

#[test]
fn world_name_is_available_only_when_unused() {
    const WORLD_A: &str = "Main";
    const WORLD_B: &str = "Arcade";
    const UNUSED: &str = "Fresh";

    let mut game = Game::default();
    game.add_world(World::new(WorldId(1), WORLD_A.to_string(), 16.0));
    game.add_world(World::new(WorldId(2), WORLD_B.to_string(), 16.0));

    assert!(!game.world_name_available(WORLD_A, None));
    assert!(game.world_name_available(UNUSED, None));
    // Renaming world 1 to its own name is allowed.
    assert!(game.world_name_available(WORLD_A, Some(WorldId(1))));
    // Renaming world 1 to world 2's name is not.
    assert!(!game.world_name_available(WORLD_B, Some(WorldId(1))));
}
