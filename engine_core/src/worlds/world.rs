use std::collections::HashMap;
use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::SpriteId;
use crate::constants::world;
use crate::tiles::tilemap::TileMap;
use crate::worlds::room::*;
use crate::worlds::room_grid::RoomGrid;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::FromInto;

/// Identifier for a world.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldId(pub usize);

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct World {
    pub id: WorldId,
    pub name: String,
    rooms: Vec<Room>,
    pub current_room_id: Option<RoomId>,
    pub starting_room_id: Option<RoomId>,
    #[serde_as(as = "Option<FromInto<[f32; 2]>>")]
    pub starting_position: Option<Vec2>,
    pub meta: WorldMeta,
    #[serde(default = "default_grid_size")]
    pub grid_size: f32,
    #[serde(skip)]
    pub room_grid: RoomGrid,
    #[serde(skip)]
    room_index: HashMap<RoomId, usize>,
}

fn default_grid_size() -> f32 {
    world::DEFAULT_GRID_SIZE
}

impl World {
    /// Returns a static dummy world used as a fallback when no real worlds exist.
    pub fn dummy() -> &'static Self {
        static DUMMY: std::sync::OnceLock<World> = std::sync::OnceLock::new();
        DUMMY.get_or_init(Self::default)
    }

    /// Rebuild the room index and grid from the current room list.
    pub fn rebuild_room_grid(&mut self) {
        self.rebuild_room_index();
        self.room_grid = RoomGrid::build(self);
    }

    /// Creates a new world with the given id, name, and grid size.
    pub fn new(id: WorldId, name: String, grid_size: f32) -> Self {
        Self {
            id,
            name,
            grid_size,
            ..Default::default()
        }
    }

    /// Creates a new world from the given id, name, rooms, and grid size.
    #[cfg(any(test, feature = "editor"))]
    pub fn from_rooms(id: WorldId, name: String, rooms: Vec<Room>, grid_size: f32) -> Self {
        let mut world = Self::new(id, name, grid_size);
        world.rooms = rooms;
        world.rebuild_room_grid();
        world
    }

    /// Returns a read-only slice of all rooms.
    pub fn rooms(&self) -> &[Room] {
        &self.rooms
    }

    /// Returns a mutable slice of all rooms (non-resizing).
    pub fn rooms_mut(&mut self) -> &mut [Room] {
        &mut self.rooms
    }

    /// Adds a room to the world and rebuilds the room grid.
    pub fn add_room(&mut self, room: Room) {
        self.rooms.push(room);
        self.rebuild_room_grid();
    }

    /// Removes a room by id and returns it. Returns None if not found.
    /// Rebuilds the room grid.
    pub fn remove_room(&mut self, room_id: RoomId) -> Option<Room> {
        let index = *self.room_index.get(&room_id)?;
        let room = self.rooms.remove(index);
        self.rebuild_room_grid();
        Some(room)
    }

    /// Rebuild the room index from the current room list.
    pub fn rebuild_room_index(&mut self) {
        self.room_index.clear();
        for (index, room) in self.rooms.iter().enumerate() {
            let previous = self.room_index.insert(room.id, index);
            debug_assert!(previous.is_none(), "duplicate RoomId {:?}", room.id);
        }
    }

    /// Returns an immutable reference to a room given its id.
    pub fn get_room(&self, id: RoomId) -> Option<&Room> {
        let index = *self.room_index.get(&id)?;
        self.rooms.get(index)
    }

    /// Returns a mutable reference to a room given its id.
    pub fn get_room_mut(&mut self, id: RoomId) -> Option<&mut Room> {
        let index = *self.room_index.get(&id)?;
        self.rooms.get_mut(index)
    }

    /// Returns an immutable reference to the current room of the world.
    pub fn current_room(&self) -> Option<&Room> {
        let id = self.current_room_id?;
        self.get_room(id)
    }

    /// Returns a mutable reference to the current room of the world.
    pub fn current_room_mut(&mut self) -> Option<&mut Room> {
        let id = self.current_room_id?;
        self.get_room_mut(id)
    }

    /// Returns the room containing this world-space position.
    pub fn room_at(&self, pos: Vec2) -> Option<RoomId> {
        self.room_grid.room_at(pos, self.grid_size)
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct WorldMeta {
    /// Position on the game map.
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub position: Vec2,
    /// Sprite of the world or None.
    pub sprite_id: Option<SpriteId>,
}

impl WorldMeta {
    /// Sets the sprite.
    pub fn set_sprite(&mut self, new_id: Option<SpriteId>, sprite_manager: &mut SpriteManager) {
        sprite_manager.change_sprite_option(&mut self.sprite_id, new_id);
    }
}

impl World {
    /// Links all exits in all rooms of this world.
    pub fn link_all_exits(&mut self) {
        let len = self.rooms().len();
        let grid_size = self.grid_size;

        for i in 0..len {
            let rooms = self.rooms_mut();
            let (left, right) = rooms.split_at_mut(i);
            let (room, right) = right.split_first_mut().unwrap();

            // Create a slice of immutable references to all other rooms
            let other_rooms: Vec<&Room> = left.iter().chain(right.iter()).collect();

            room.link_exits(&other_rooms, grid_size);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos(pub IVec2);

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        GridPos(IVec2::new(x, y))
    }

    pub fn x(&self) -> i32 {
        self.0.x
    }
    pub fn y(&self) -> i32 {
        self.0.y
    }

    /// Check if this position is within map bounds
    pub fn is_in_bounds(&self, width: usize, height: usize) -> bool {
        self.0.x >= 0 && self.0.y >= 0 && self.0.x < width as i32 && self.0.y < height as i32
    }

    /// Convert from world coordinates to tile coordinates.
    pub fn from_world(world_pos: Vec2, grid_size: f32) -> Self {
        GridPos::new(
            (world_pos.x / grid_size) as i32,
            (world_pos.y / grid_size) as i32,
        )
    }

    /// Convert to usize tuple (if valid)
    pub fn as_usize(&self) -> Option<(usize, usize)> {
        if self.0.x >= 0 && self.0.y >= 0 {
            Some((self.0.x as usize, self.0.y as usize))
        } else {
            None
        }
    }

    /// Convert from world coordinates to tile coordinates, snapping to map edges.
    pub fn from_world_edge(world_pos: Vec2, map: &TileMap, grid_size: f32) -> Self {
        let mut x = (world_pos.x / grid_size).floor() as i32;
        let mut y = (world_pos.y / grid_size).floor() as i32;

        // Snap to map edges
        if x < 0 {
            x = -1;
        } else if x >= map.width as i32 {
            x = map.width as i32;
        }

        if y < 0 {
            y = -1;
        } else if y >= map.height as i32 {
            y = map.height as i32;
        }

        GridPos::new(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(id: usize) -> Room {
        Room {
            id: RoomId(id),
            ..Default::default()
        }
    }

    #[test]
    fn room_index_get_room_returns_room_after_rebuild() {
        let mut world = World::from_rooms(WorldId(1), "test".to_string(), vec![room(1), room(2)], 16.0);
        world.rebuild_room_index();

        assert_eq!(world.get_room(RoomId(2)).map(|room| room.id), Some(RoomId(2)));
    }

    #[test]
    fn room_index_get_room_mut_tracks_swap_remove_after_rebuild() {
        let mut world = World::from_rooms(WorldId(1), "test".to_string(), vec![room(1), room(2)], 16.0);
        world.rooms.swap_remove(0);
        world.rebuild_room_index();

        assert_eq!(world.get_room_mut(RoomId(2)).map(|room| room.id), Some(RoomId(2)));
        assert!(world.get_room(RoomId(1)).is_none());
    }

    #[test]
    fn room_index_current_room_uses_indexed_lookup() {
        let mut world = World::from_rooms(WorldId(1), "test".to_string(), vec![room(1), room(2)], 16.0);
        world.current_room_id = Some(RoomId(2));
        world.rebuild_room_index();

        assert_eq!(world.current_room().map(|room| room.id), Some(RoomId(2)));
    }
}
