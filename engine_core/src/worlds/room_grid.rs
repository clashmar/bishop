use std::collections::HashMap;
use bishop::prelude::*;
use crate::worlds::room::*;
use crate::worlds::room_bounds::*;
use crate::worlds::World;

/// Maps grid cells to the room that owns them.
#[derive(Clone, Debug, Default)]
pub struct RoomGrid {
    cells: HashMap<(i32, i32), RoomId>,
}

impl RoomGrid {
    /// Build the grid from a world's rooms.
    pub fn build(world: &World) -> Self {
        let mut cells = HashMap::new();
        for room in world.rooms() {
            let bounds = RoomBounds::from_room(room, world.grid_size);
            for (cx, cy) in bounds.cells() {
                cells.insert((cx, cy), room.id);
            }
        }
        Self { cells }
    }

    /// Look up the room containing a world-space position.
    pub fn room_at(&self, pos: Vec2, grid_size: f32) -> Option<RoomId> {
        let cx = (pos.x / grid_size).floor() as i32;
        let cy = (pos.y / grid_size).floor() as i32;
        self.cells.get(&(cx, cy)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::test_utils::make_room;
    use crate::worlds::{World, WorldId};

    fn make_world(rooms: Vec<Room>, grid_size: f32) -> World {
        World::from_rooms(WorldId(0), String::new(), rooms, grid_size)
    }

    #[test]
    fn build_empty_rooms() {
        let world = make_world(vec![], 1.0);
        let grid = RoomGrid::build(&world);
        assert!(grid.cells.is_empty());
        assert_eq!(grid.room_at(Vec2::new(0.0, 0.0), 1.0), None);
    }

    #[test]
    fn build_single_room() {
        let world = make_world(vec![make_room(Some(42), 0.0, 0.0, 3.0, 2.0)], 1.0);
        let grid = RoomGrid::build(&world);
        // Inside room
        assert_eq!(grid.room_at(Vec2::new(1.5, 1.5), 1.0), Some(RoomId(42)));
        assert_eq!(grid.room_at(Vec2::new(0.1, 0.1), 1.0), Some(RoomId(42)));
        // Outside room
        assert_eq!(grid.room_at(Vec2::new(5.0, 5.0), 1.0), None);
        assert_eq!(grid.room_at(Vec2::new(-1.0, -1.0), 1.0), None);
    }

    #[test]
    fn build_two_non_overlapping_rooms() {
        let world = make_world(
            vec![
                make_room(Some(1), 0.0, 0.0, 3.0, 3.0),
                make_room(Some(2), 5.0, 5.0, 2.0, 2.0),
            ],
            1.0,
        );
        let grid = RoomGrid::build(&world);
        assert_eq!(grid.room_at(Vec2::new(1.0, 1.0), 1.0), Some(RoomId(1)));
        assert_eq!(grid.room_at(Vec2::new(6.0, 6.0), 1.0), Some(RoomId(2)));
        // Gap between rooms
        assert_eq!(grid.room_at(Vec2::new(4.0, 4.0), 1.0), None);
    }

    #[test]
    fn room_at_non_one_grid_size() {
        let world = make_world(vec![make_room(Some(10), 16.0, 16.0, 2.0, 2.0)], 8.0);
        let grid = RoomGrid::build(&world);
        // room spans cells (2,2) to (3,3)
        assert_eq!(grid.room_at(Vec2::new(20.0, 20.0), 8.0), Some(RoomId(10)));
        assert_eq!(grid.room_at(Vec2::new(0.0, 0.0), 8.0), None);
    }

    #[test]
    fn room_at_boundary_between_rooms() {
        let world = make_world(
            vec![
                make_room(Some(1), 0.0, 0.0, 3.0, 3.0),
                make_room(Some(2), 3.0, 0.0, 3.0, 3.0),
            ],
            1.0,
        );
        let grid = RoomGrid::build(&world);
        // Just inside room 1 (max-x exclusive)
        assert_eq!(grid.room_at(Vec2::new(2.9, 1.0), 1.0), Some(RoomId(1)));
        // Exactly at room 1 max-x = 3.0 — exclusive, should be room 2
        assert_eq!(grid.room_at(Vec2::new(3.0, 1.0), 1.0), Some(RoomId(2)));
    }
}
