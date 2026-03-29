// engine_core/src/world/room.rs
use crate::camera::game_camera::RoomCamera;
use crate::constants::*;
use crate::ecs::component::*;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::transform::*;
use crate::tiles::tilemap::TileMap;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::FromInto;
use serde_with::serde_as;
use std::collections::HashSet;

/// Identifier for a room, globally unique across all worlds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct RoomId(pub usize);

impl std::ops::Deref for RoomId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RoomId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub position: Vec2, // Top-left origin in pixels
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub size: Vec2,
    pub exits: Vec<Exit>,
    pub adjacent_rooms: Vec<RoomId>,
    pub variants: Vec<RoomVariant>,
    pub darkness: f32,
}

impl Room {
    /// Creates a new room with the given pre-allocated room ID.
    pub fn new(ecs: &mut Ecs, room_id: RoomId, grid_size: f32) -> Self {
        let first_variant = RoomVariant {
            id: "default".to_string(),
            tilemap: TileMap::new(DEFAULT_ROOM_SIZE.x as usize, DEFAULT_ROOM_SIZE.y as usize),
        };

        let room = Room {
            id: room_id,
            name: "untitled".to_string(),
            position: DEFAULT_ROOM_POSITION,
            size: DEFAULT_ROOM_SIZE,
            exits: vec![],
            adjacent_rooms: vec![],
            variants: vec![first_variant],
            darkness: 0.,
        };

        room.create_room_camera(ecs, room_id, grid_size);
        room
    }

    /// Link exits to adjacent rooms based on their positions.
    pub fn link_exits(&mut self, other_rooms: &[&Room], grid_size: f32) {
        let epsilon = 0.01; // tolerance for floating-point comparisons

        for exit in self.exits.iter_mut() {
            exit.target_room_id = None;

            // Local to world position
            let exit_world_pos = (self.position / grid_size) + exit.position;

            'other_rooms: for other_room in other_rooms.iter() {
                for other_exit in &other_room.exits {
                    // World position of the other room's exit
                    let other_world_pos = (other_room.position / grid_size) + other_exit.position;

                    let linked = match exit.direction {
                        ExitDirection::Up => {
                            other_exit.direction == ExitDirection::Down
                                && (exit_world_pos.y - (other_world_pos.y - 1.0)).abs() < epsilon
                                && (exit_world_pos.x - other_world_pos.x).abs() < epsilon
                        }
                        ExitDirection::Down => {
                            other_exit.direction == ExitDirection::Up
                                && (exit_world_pos.y - 1.0 - other_world_pos.y).abs() < epsilon
                                && (exit_world_pos.x - other_world_pos.x).abs() < epsilon
                        }
                        ExitDirection::Left => {
                            other_exit.direction == ExitDirection::Right
                                && (exit_world_pos.x - other_world_pos.x + 1.0).abs() < epsilon
                                && (exit_world_pos.y - other_world_pos.y).abs() < epsilon
                        }
                        ExitDirection::Right => {
                            other_exit.direction == ExitDirection::Left
                                && (exit_world_pos.x - other_world_pos.x - 1.0).abs() < epsilon
                                && (exit_world_pos.y - other_world_pos.y).abs() < epsilon
                        }
                    };

                    if linked {
                        exit.target_room_id = Some(other_room.id);
                        break 'other_rooms;
                    }
                }
            }
        }
    }

    /// Returns the world exit positions for this room.
    pub fn world_exit_positions(&self, grid_size: f32) -> Vec<(Vec2, ExitDirection)> {
        self.exits
            .iter()
            .map(|exit| (self.position / grid_size + exit.position, exit.direction))
            .collect()
    }

    /// Returns exits from this room that face toward the target room.
    /// Only returns exits if the rooms are truly adjacent (sharing an edge).
    pub fn exits_facing_room(&self, target: &Room, grid_size: f32) -> Vec<(Vec2, ExitDirection)> {
        let self_min = self.position;
        let self_max = self.position + self.size * grid_size;
        let target_min = target.position;
        let target_max = target.position + target.size * grid_size;

        let epsilon = 0.01;

        // Check for overlap on each axis (rooms must overlap on perpendicular axis to be adjacent)
        let x_overlap = self_min.x < target_max.x && self_max.x > target_min.x;
        let y_overlap = self_min.y < target_max.y && self_max.y > target_min.y;

        // Determine which edge of self faces target
        // Rooms must touch on one axis AND overlap on the perpendicular axis
        let facing_direction = if (self_max.y - target_min.y).abs() < epsilon && x_overlap {
            // Self's bottom edge touches target's top edge
            Some(ExitDirection::Down)
        } else if (self_min.y - target_max.y).abs() < epsilon && x_overlap {
            // Self's top edge touches target's bottom edge
            Some(ExitDirection::Up)
        } else if (self_max.x - target_min.x).abs() < epsilon && y_overlap {
            // Self's right edge touches target's left edge
            Some(ExitDirection::Right)
        } else if (self_min.x - target_max.x).abs() < epsilon && y_overlap {
            // Self's left edge touches target's right edge
            Some(ExitDirection::Left)
        } else {
            None
        };

        let facing = match facing_direction {
            Some(d) => d,
            None => return vec![],
        };

        self.exits
            .iter()
            .filter(|exit| exit.direction == facing)
            .map(|exit| {
                let world_pos = self.position / grid_size + exit.position;
                (world_pos, exit.direction)
            })
            .collect()
    }

    pub fn create_room_camera(&self, ecs: &mut Ecs, room_id: RoomId, grid_size: f32) {
        const CAMERA_PREFIX: &str = "Camera ";
        let name_store = ecs.get_store::<Name>();
        let cur_room_store = ecs.get_store::<CurrentRoom>();

        let mut used: HashSet<usize> = HashSet::new();

        for (entity, name) in name_store.data.iter() {
            if let Some(cur_room) = cur_room_store.get(*entity) {
                if cur_room.0 != self.id {
                    continue;
                }
                if let Some(num_str) = name.strip_prefix(CAMERA_PREFIX)
                    && let Ok(num) = num_str.parse::<usize>()
                    && num > 0
                {
                    used.insert(num);
                }
            }
        }

        let mut next_idx = 1;
        while used.contains(&next_idx) {
            next_idx += 1;
        }

        ecs.create_entity()
            .with(Transform {
                position: self.position,
                pivot: Pivot::CenterLeft,
                ..Default::default()
            })
            .with(RoomCamera::new(room_id, grid_size))
            .with(CurrentRoom(self.id))
            .with(Name(format!("{}{}", CAMERA_PREFIX, next_idx)));
    }

    /// Returns the axis‑aligned rectangle that a room occupies in world space.
    #[inline]
    pub fn room_bounds(&self, grid_size: f32) -> (Vec2, Vec2) {
        let min = self.position;
        let max = self.position + self.size * grid_size;
        (min, max)
    }

    /// Returns the index of the current variant.
    pub fn current_variant_index(&self) -> usize {
        0
    }

    /// Returns a reference to the current variant of the room.
    pub fn current_variant(&self) -> &RoomVariant {
        &self.variants[self.current_variant_index()]
    }

    /// Returns a mutable reference to the current variant of the room.
    pub fn current_variant_mut(&mut self) -> &mut RoomVariant {
        let idx = self.current_variant_index();
        &mut self.variants[idx]
    }
}

/// Returns a HashSet of all entities in the current room.
pub fn entities_in_room(ecs: &mut Ecs, room_id: RoomId) -> HashSet<Entity> {
    let room_store = ecs.get_store::<CurrentRoom>();
    room_store
        .data
        .iter()
        .filter_map(|(entity, cur_room)| {
            if cur_room.0 == room_id {
                Some(*entity)
            } else {
                None
            }
        })
        .collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RoomVariant {
    pub id: String,
    pub tilemap: TileMap,
}

impl Default for RoomVariant {
    fn default() -> Self {
        Self {
            id: String::new(),
            tilemap: TileMap::new(10, 10),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitDirection {
    #[default]
    Up,
    Right,
    Down,
    Left,
}

impl ExitDirection {
    /// Returns the opposite direction.
    pub fn opposite(&self) -> Self {
        match self {
            ExitDirection::Up => ExitDirection::Down,
            ExitDirection::Down => ExitDirection::Up,
            ExitDirection::Left => ExitDirection::Right,
            ExitDirection::Right => ExitDirection::Left,
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, Default)]
#[serde(default)]
pub struct Exit {
    #[serde_as(as = "FromInto<[f32; 2]>")]
    // Local grid coordinate
    pub position: Vec2,
    pub direction: ExitDirection,
    pub target_room_id: Option<RoomId>,
}
