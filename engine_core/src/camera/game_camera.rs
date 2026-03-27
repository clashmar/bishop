// engine_core/src/camera/game_camera.rs
use crate::ecs::component::CurrentRoom;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::transform::Transform;
use crate::engine_global::cam_tile_dims;
use crate::worlds::room::RoomId;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};
use std::fmt;
use strum_macros::EnumIter;

#[derive(Debug, Default)]
pub struct GameCamera {
    pub camera: Camera2D,
    pub id: usize,
    /// The camera entity's original transform position (used for clamped follow modes).
    pub origin: Vec2,
}

impl Clone for GameCamera {
    fn clone(&self) -> Self {
        Self {
            camera: Camera2D {
                target: self.camera.target,
                zoom: self.camera.zoom,
                rotation: self.camera.rotation,
                offset: self.camera.offset,
                render_target: self.camera.render_target.clone(),
                ..Default::default()
            },
            id: self.id,
            origin: self.origin,
        }
    }
}

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
    #[serde_as(as = "FromInto<[f32; 2]>")]
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

/// Creates a render target sized for the given grid size.
pub fn game_render_target<C: BishopContext>(ctx: &mut C, grid_size: f32) -> BishopRenderTarget {
    let width = world_virtual_width(grid_size) as u32;
    let height = world_virtual_height(grid_size) as u32;
    ctx.create_render_target(width, height)
}

/// Returns every `GameCamera` for a room from its id.
pub fn get_room_cameras(ecs: &Ecs, room_id: RoomId) -> Vec<(Entity, RoomCamera)> {
    let cam_store = ecs.get_store::<RoomCamera>();
    let room_store = ecs.get_store::<CurrentRoom>();

    cam_store
        .data
        .iter()
        .filter_map(|(entity, room_cam)| {
            let cur = room_store.get(*entity)?;
            if cur.0 != room_id {
                return None;
            }
            Some((*entity, *room_cam))
        })
        .collect()
}

/// Converts a `RoomCamera` component into a `GameCamera` from its Entity.
/// The render target is not set here; callers must assign it after selecting the active camera.
pub fn room_to_game_camera(
    ecs: &Ecs,
    entity: &Entity,
    room_camera: &RoomCamera,
    player_pos: Vec2,
) -> GameCamera {
    let pos_store = ecs.get_store::<Transform>();
    let origin = pos_store
        .data
        .get(entity)
        .expect("Camera should always have a Transform component")
        .position;

    let target = match room_camera.camera_mode {
        CameraMode::Fixed => origin,
        CameraMode::Follow(FollowRestriction::Free) => player_pos,
        CameraMode::Follow(FollowRestriction::ClampX) => Vec2::new(origin.x, player_pos.y),
        CameraMode::Follow(FollowRestriction::ClampY) => Vec2::new(player_pos.x, origin.y),
    };

    let camera = Camera2D {
        target,
        zoom: room_camera.zoom,
        ..Default::default()
    };

    GameCamera {
        camera,
        id: entity.0,
        origin,
    }
}

/// Returns a `GameCamera` for a room by its entity id.
/// If the id is None or not found, returns the first camera in the room.
pub fn get_room_camera_by_id<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    room_id: RoomId,
    grid_size: f32,
    camera_id: Option<usize>,
) -> Option<GameCamera> {
    let trans_store = ecs.get_store::<Transform>();
    let room_cameras = get_room_cameras(ecs, room_id);

    if room_cameras.is_empty() {
        return None;
    }

    let index = match camera_id {
        Some(id) => room_cameras
            .iter()
            .position(|(e, _)| e.0 == id)
            .unwrap_or(0),
        None => 0,
    };

    let (entity, room_cam) = &room_cameras[index];
    let origin = trans_store.data.get(entity)?.position;

    let camera = Camera2D {
        target: origin,
        zoom: room_cam.zoom,
        render_target: Some(game_render_target(ctx, grid_size)),
        ..Default::default()
    };

    Some(GameCamera {
        camera,
        id: entity.0,
        origin,
    })
}

/// Returns the next `GameCamera` for a room, cycling through all available cameras.
/// If `current_id` is None or not found, returns the first camera.
pub fn get_next_room_camera(
    ctx: &mut impl BishopContext,
    ecs: &Ecs,
    room_id: RoomId,
    grid_size: f32,
    current_id: Option<usize>,
) -> Option<GameCamera> {
    let trans_store = ecs.get_store::<Transform>();
    let room_cameras = get_room_cameras(ecs, room_id);

    if room_cameras.is_empty() {
        return None;
    }

    let next_index = match current_id {
        Some(id) => {
            let current_index = room_cameras.iter().position(|(e, _)| e.0 == id);
            match current_index {
                Some(idx) => (idx + 1) % room_cameras.len(),
                None => 0,
            }
        }
        None => 0,
    };

    let (entity, room_cam) = &room_cameras[next_index];
    let origin = trans_store.data.get(entity)?.position;

    let camera = Camera2D {
        target: origin,
        zoom: room_cam.zoom,
        render_target: Some(game_render_target(ctx, grid_size)),
        ..Default::default()
    };

    Some(GameCamera {
        camera,
        id: entity.0,
        origin,
    })
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
