// engine_core/src/camera/game_camera.rs
use crate::ecs::components::room_camera::{
    world_virtual_height, world_virtual_width, CameraMode, FollowRestriction, RoomCamera,
};
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::Transform;
use crate::worlds::room::RoomId;
use bishop::prelude::*;

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

/// Creates a render target sized for the given grid size.
pub fn game_render_target<C: BishopContext>(ctx: &mut C, grid_size: f32) -> BishopRenderTarget {
    let width = world_virtual_width(grid_size) as u32;
    let height = world_virtual_height(grid_size) as u32;
    ctx.create_render_target(width, height)
}

/// Returns every `GameCamera` for a room from its id.
pub fn get_room_cameras(ecs: &Ecs, room_id: RoomId) -> Vec<(Entity, RoomCamera)> {
    let cam_store = ecs.get_store::<RoomCamera>();

    ecs.entities_in_room(room_id)
        .iter()
        .filter_map(|entity| {
            ecs.assert_room_membership(room_id, *entity);
            cam_store.get(*entity).copied().map(|room_cam| (*entity, room_cam))
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
