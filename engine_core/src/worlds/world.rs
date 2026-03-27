// engine_core/src/worlds/world.rs
use crate::assets::asset_manager::AssetManager;
use crate::assets::sprite::SpriteId;
use crate::tiles::tilemap::TileMap;
use crate::worlds::room::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::FromInto;
use serde_with::serde_as;
use uuid::Uuid;

/// Identifier for a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct WorldId(pub Uuid);

/// Default grid size in pixels.
fn default_grid_size() -> f32 {
    16.0
}

#[serde_as]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct World {
    pub id: WorldId,
    pub name: String,
    pub rooms: Vec<Room>,
    pub current_room_id: Option<RoomId>,
    pub starting_room_id: Option<RoomId>,
    #[serde_as(as = "Option<FromInto<[f32; 2]>>")]
    pub starting_position: Option<Vec2>,
    /// Meta information about the world to display the world in the editor/game.
    pub meta: WorldMeta,
    /// Grid size in pixels for this world.
    #[serde(default = "default_grid_size")]
    pub grid_size: f32,
}

#[serde_as]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WorldMeta {
    /// Position on the game map.
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub position: Vec2,
    /// Sprite of the world or None.
    pub sprite_id: Option<SpriteId>,
}

impl WorldMeta {
    /// Sets the sprite, handling ref counting for the change.
    pub fn set_sprite(&mut self, new_id: Option<SpriteId>, asset_manager: &mut AssetManager) {
        asset_manager.change_sprite_option(&mut self.sprite_id, new_id);
    }
}

impl World {
    /// Links all exits in all rooms of this world.
    pub fn link_all_exits(&mut self) {
        let len = self.rooms.len();
        let grid_size = self.grid_size;

        for i in 0..len {
            // Split the rooms vector into two disjoint mutable slices
            let (left, right) = self.rooms.split_at_mut(i);
            let (room, right) = right.split_first_mut().unwrap(); // room = &mut self.rooms[i]

            // Create a slice of immutable references to all other rooms
            let other_rooms: Vec<&Room> = left.iter().chain(right.iter()).collect();

            room.link_exits(&other_rooms, grid_size);
        }
    }

    /// Returns an immutable reference to a room given its id.
    pub fn get_room(&self, id: RoomId) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    /// Returns a mutable reference to a room given its id.
    pub fn get_room_mut(&mut self, id: RoomId) -> Option<&mut Room> {
        self.rooms.iter_mut().find(|r| r.id == id)
    }

    /// Returns an  immutable reference to the current room of the world.
    pub fn current_room(&self) -> Option<&Room> {
        let id = self.current_room_id?;
        self.get_room(id)
    }

    /// Returns a mutable reference to the current room of the world.
    pub fn current_room_mut(&mut self) -> Option<&mut Room> {
        let id = self.current_room_id?;
        self.get_room_mut(id)
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
