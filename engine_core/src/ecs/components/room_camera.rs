use crate::engine_global::cam_tile_dims;
use crate::worlds::room::RoomId;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;
use strum_macros::EnumIter;

/// Returns the virtual width in pixels for the given grid size.
pub fn world_virtual_width(grid_size: f32) -> f32 {
    cam_tile_dims().0 * grid_size
}

/// Returns the virtual height in pixels for the given grid size.
pub fn world_virtual_height(grid_size: f32) -> f32 {
    cam_tile_dims().1 * grid_size
}

/// Component for a room camera used by the game.
#[ecs_component]
#[serde_as]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct RoomCamera {
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub zoom: Vec2,
    pub room_id: RoomId,
    pub zoom_mode: ZoomMode,
    pub camera_mode: CameraMode,
}

impl RoomCamera {
    /// Creates a new RoomCamera with the world grid size.
    pub fn new(room_id: RoomId, grid_size: f32) -> Self {
        let zoom = Vec2::new(
            1.0 / world_virtual_width(grid_size) * 2.0,
            1.0 / world_virtual_height(grid_size) * 2.0,
        );
        RoomCamera {
            zoom,
            room_id,
            zoom_mode: ZoomMode::Step,
            camera_mode: CameraMode::Fixed,
        }
    }

    /// Creates a new RoomCamera with zoom calculated for the given grid size.
    pub fn with_grid_size(room_id: RoomId, grid_size: f32) -> Self {
        let zoom = Vec2::new(
            1.0 / world_virtual_width(grid_size) * 2.0,
            1.0 / world_virtual_height(grid_size) * 2.0,
        );
        RoomCamera {
            zoom,
            room_id,
            zoom_mode: ZoomMode::Step,
            camera_mode: CameraMode::Fixed,
        }
    }
}

/// The two display modes the inspector can use.
#[derive(EnumIter, Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum ZoomMode {
    #[default]
    Step,
    Free,
}

impl ZoomMode {
    pub fn ui_label(&self) -> String {
        match *self {
            ZoomMode::Step => "Step".to_string(),
            ZoomMode::Free => "Free".to_string(),
        }
    }
}

impl fmt::Display for ZoomMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ui_label())
    }
}

/// The two display modes the inspector can use.
#[derive(EnumIter, Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum CameraMode {
    #[default]
    Fixed,
    /// The camera is set to follow the player, with optional restrictions.
    Follow(FollowRestriction),
}

impl CameraMode {
    pub fn ui_label(&self) -> String {
        match self {
            &CameraMode::Fixed => "Fixed".to_string(),
            CameraMode::Follow(restriction) => format!("Follow ({})", restriction),
        }
    }
}

impl fmt::Display for CameraMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ui_label())
    }
}

/// The possible restrictions for Follow mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FollowRestriction {
    /// Camera can move freely in all directions.
    #[default]
    Free,
    /// The camera is clamped vertically.
    ClampY,
    /// The camera is clamped horizontally.
    ClampX,
}

impl fmt::Display for FollowRestriction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let txt = match self {
            FollowRestriction::Free => "Free",
            FollowRestriction::ClampY => "Clamp Y",
            FollowRestriction::ClampX => "Clamp X",
        };
        write!(f, "{}", txt)
    }
}

/// Compute zoom vector from a scalar value.
pub fn zoom_from_scalar(scalar: f32, grid_size: f32) -> Vec2 {
    // Fixed virtual aspect
    let aspect = world_virtual_width(grid_size) / world_virtual_height(grid_size);

    if aspect >= 1.0 {
        Vec2::new(scalar / aspect, scalar)
    } else {
        Vec2::new(scalar, scalar * aspect)
    }
}
