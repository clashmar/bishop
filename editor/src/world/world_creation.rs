use engine_core::constants::world as world_constants;
use engine_core::ecs::{AudioSource, Name, PlayerProxy, Singleton, Transform, WorldEntry};
use engine_core::game::Game;
use engine_core::worlds::{Room, World, WorldMeta, WorldVisual};

/// Create a fresh world with a single default room.
pub fn create_new_world(game: &mut Game) -> World {
    let id = game.id_allocator.allocate_world_id();
    let name = "new".to_string();
    let room_id = game.id_allocator.allocate_room_id();
    let first_room = Room::new(&mut game.ecs, room_id, world_constants::DEFAULT_GRID_SIZE);
    let room_origin = first_room.position;

    let mut world = World::new(id, name.clone(), world_constants::DEFAULT_GRID_SIZE);
    world.add_room(first_room);
    world.current_room_id = None;
    world.meta = WorldMeta::default();
    world.meta.visual = WorldVisual::Color(WorldVisual::random_color());

    let _spawn_point = game
        .ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: room_origin,
            ..Default::default()
        })
        .with(Name("Player Proxy".to_string()))
        .with_current_room(room_id)
        .finish();

    let world_singleton = game
        .ecs
        .create_entity()
        .with(Singleton)
        .with(AudioSource::default())
        .with_current_room(room_id)
        .finish();
    world.singleton = world_singleton;

    // Default start entry point
    game.ecs
        .create_entity()
        .with(WorldEntry { name: String::new(), is_start: true })
        .with(Transform {
            position: room_origin,
            ..Default::default()
        })
        .with_current_room(room_id)
        .finish();

    world
}

#[cfg(test)]
mod tests {
    use super::create_new_world;
    use engine_core::game::Game;
    use engine_core::worlds::WorldVisual;

    #[test]
    fn new_world_visual_uses_a_random_opaque_color() {
        let mut game = Game::default();

        let world = create_new_world(&mut game);

        match world.meta.visual {
            WorldVisual::Color(color) => {
                assert!((0.0..=1.0).contains(&color.r));
                assert!((0.0..=1.0).contains(&color.g));
                assert!((0.0..=1.0).contains(&color.b));
                assert_eq!(color.a, 1.0);
            }
            WorldVisual::Image(_) => panic!("new world must begin with a color visual"),
        }
    }
}
