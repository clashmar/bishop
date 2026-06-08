use crate::game::Game;
use crate::worlds::room::RoomId;
use crate::worlds::world::WorldId;

#[derive(Debug)]
pub struct IdAllocator {
    next_world_id: usize,
    next_room_id: usize,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self {
            next_world_id: 1,
            next_room_id: 1,
        }
    }
}

impl IdAllocator {
    pub fn from_game(game: &Game) -> Self {
        let next_world_id = game.worlds().iter().map(|w| w.id.0).max().unwrap_or(0) + 1;
        let next_room_id = game
            .worlds()
            .iter()
            .flat_map(|w| w.rooms().iter().map(|r| r.id.0))
            .max()
            .unwrap_or(0)
            + 1;
        Self {
            next_world_id,
            next_room_id,
        }
    }

    pub fn allocate_world_id(&mut self) -> WorldId {
        let id = WorldId(self.next_world_id);
        self.next_world_id += 1;
        id
    }

    pub fn allocate_room_id(&mut self) -> RoomId {
        let id = RoomId(self.next_room_id);
        self.next_room_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_world_id_starts_at_1_after_from_game() {
        let game = Game::default();
        let mut alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.allocate_world_id(), WorldId(1));
    }

    #[test]
    fn allocate_world_id_increments() {
        let game = Game::default();
        let mut alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.allocate_world_id(), WorldId(1));
        assert_eq!(alloc.allocate_world_id(), WorldId(2));
    }

    #[test]
    fn allocate_room_id_starts_at_1_after_from_game() {
        let game = Game::default();
        let mut alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.allocate_room_id(), RoomId(1));
    }

    #[test]
    fn allocate_room_id_increments() {
        let game = Game::default();
        let mut alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.allocate_room_id(), RoomId(1));
        assert_eq!(alloc.allocate_room_id(), RoomId(2));
    }

    #[test]
    fn from_game_computes_next_world_id_from_max() {
        use crate::worlds::world::World;

        let mut game = Game::default();
        game.add_world(World::new(WorldId(2), String::new(), 16.0));
        game.add_world(World::new(WorldId(5), String::new(), 16.0));
        let alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.next_world_id, 6);
    }

    #[test]
    fn from_game_computes_next_room_id_from_max() {
        use crate::worlds::room::Room;
        use crate::worlds::world::World;

        let mut game = Game::default();
        game.add_world(World::from_rooms(
            WorldId(1),
            String::new(),
            vec![Room {
                id: RoomId(3),
                ..Default::default()
            }],
            16.0,
        ));
        game.add_world(World::from_rooms(
            WorldId(2),
            String::new(),
            vec![Room {
                id: RoomId(7),
                ..Default::default()
            }],
            16.0,
        ));
        let alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.next_room_id, 8);
    }

    #[test]
    fn from_game_handles_empty_game() {
        let game = Game::default();
        let alloc = IdAllocator::from_game(&game);
        assert_eq!(alloc.next_world_id, 1);
        assert_eq!(alloc.next_room_id, 1);
    }
}
