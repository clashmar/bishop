use bishop::prelude::*;
use crate::worlds::room::Room;

/// A room's extent in grid cell coordinates.
#[derive(Clone, Debug)]
pub struct RoomBounds {
    /// Minimum grid cell (inclusive).
    pub min: Vec2,
    /// Maximum grid cell (inclusive).
    pub max: Vec2,
}

impl RoomBounds {
    /// Compute grid cell extent from a room's position and size.
    pub fn from_room(room: &Room, grid_size: f32) -> Self {
        let min_x = (room.position.x / grid_size).floor();
        let min_y = (room.position.y / grid_size).floor();
        let max_x = ((room.position.x + room.size.x * grid_size) / grid_size).floor() - 1.0;
        let max_y = ((room.position.y + room.size.y * grid_size) / grid_size).floor() - 1.0;
        Self {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }

    /// Check whether a world-space position falls within this room.
    /// Boundary semantics: x ∈ [min.x, max.x + 1), y ∈ (min.y, max.y + 1].
    pub fn contains(&self, pos: Vec2, grid_size: f32) -> bool {
        let cx = (pos.x / grid_size).floor();
        let cy = (pos.y / grid_size).floor();
        cx >= self.min.x && cx < self.max.x + 1.0
            && cy > self.min.y && cy <= self.max.y + 1.0
    }

    /// Iterate all grid cells in this extent.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> {
        let min_x = self.min.x as i32;
        let min_y = self.min.y as i32;
        let max_x = self.max.x as i32;
        let max_y = self.max.y as i32;
        (min_x..=max_x).flat_map(move |x| (min_y..=max_y).map(move |y| (x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::test_utils::make_room;

    #[test]
    fn from_room_standard_grid() {
        let room = make_room(None, 10.0, 20.0, 3.0, 2.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        assert_eq!(bounds.min, Vec2::new(10.0, 20.0));
        assert_eq!(bounds.max, Vec2::new(12.0, 21.0));
    }

    #[test]
    fn from_room_non_one_grid() {
        let room = make_room(None, 24.0, 16.0, 4.0, 2.0);
        let bounds = RoomBounds::from_room(&room, 8.0);
        assert_eq!(bounds.min, Vec2::new(3.0, 2.0));
        assert_eq!(bounds.max, Vec2::new(6.0, 3.0));
    }

    #[test]
    fn from_room_negative_position() {
        let room = Room {
            position: Vec2::new(-16.0, -8.0),
            size: Vec2::new(2.0, 2.0),
            ..Default::default()
        };
        let bounds = RoomBounds::from_room(&room, 8.0);
        assert_eq!(bounds.min, Vec2::new(-2.0, -1.0));
        assert_eq!(bounds.max, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn contains_interior_point() {
        let room = make_room(None, 0.0, 0.0, 5.0, 5.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        assert!(bounds.contains(Vec2::new(2.5, 2.5), 1.0));
    }

    #[test]
    fn contains_at_min_edge() {
        let room = make_room(None, 0.0, 0.0, 5.0, 5.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        // min-x inclusive: pos.x exactly at room.position.x → true
        assert!(bounds.contains(Vec2::new(0.0, 2.5), 1.0));
        // min-y exclusive: pos.y exactly at room.position.y → false
        assert!(!bounds.contains(Vec2::new(2.5, 0.0), 1.0));
    }

    #[test]
    fn contains_at_max_edge() {
        let room = make_room(None, 0.0, 0.0, 5.0, 5.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        // max-x exclusive: pos.x exactly at max → false
        assert!(!bounds.contains(Vec2::new(5.0, 2.5), 1.0));
        // max-y inclusive: pos.y exactly at max → true
        assert!(bounds.contains(Vec2::new(2.5, 5.0), 1.0));
    }

    #[test]
    fn contains_outside() {
        let room = make_room(None, 10.0, 10.0, 5.0, 5.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        assert!(!bounds.contains(Vec2::new(0.0, 12.0), 1.0));
        assert!(!bounds.contains(Vec2::new(12.0, 20.0), 1.0));
    }

    #[test]
    fn cells_iterator_count() {
        let room = make_room(None, 0.0, 0.0, 3.0, 2.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        let cells: Vec<_> = bounds.cells().collect();
        assert_eq!(cells.len(), 6);
    }

    #[test]
    fn cells_iterator_correct_coordinates() {
        let room = make_room(None, 0.0, 0.0, 2.0, 2.0);
        let bounds = RoomBounds::from_room(&room, 1.0);
        let mut cells: Vec<_> = bounds.cells().collect();
        cells.sort();
        assert_eq!(
            cells,
            vec![(0, 0), (0, 1), (1, 0), (1, 1)]
        );
    }
}
