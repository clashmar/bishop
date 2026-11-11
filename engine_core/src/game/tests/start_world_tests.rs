use crate::game::Game;
use crate::worlds::{World, WorldId};

#[test]
fn browsing_worlds_preserves_the_configured_start_world() {
    let mut game = Game::default();
    let first = WorldId(1);
    let second = WorldId(2);
    game.add_world(World::new(first, "First".to_string(), 16.0));
    game.add_world(World::new(second, "Second".to_string(), 16.0));

    assert_eq!(game.start_world_id, Some(first));
    game.select_world(second);
    assert_eq!(game.start_world_id, Some(first));
    game.set_start_world(second).unwrap();
    game.select_world(first);
    assert_eq!(game.current_world_id, Some(first));
    assert_eq!(game.start_world_id, Some(second));
    assert!(game.set_start_world(WorldId(99)).is_err());
    assert_eq!(game.start_world_id, Some(second));
}

#[cfg(feature = "editor")]
#[test]
fn deleting_the_start_world_selects_a_remaining_world() {
    let mut game = Game::default();
    let first = WorldId(1);
    let second = WorldId(2);
    game.add_world(World::new(first, "First".to_string(), 16.0));
    game.add_world(World::new(second, "Second".to_string(), 16.0));
    game.delete_world(first);
    assert_eq!(game.start_world_id, Some(second));
    game.delete_world(second);
    assert_eq!(game.start_world_id, None);
}
