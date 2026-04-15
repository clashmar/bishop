// engine_core/src/camera/camera_manager.rs
use crate::camera::game_camera::*;
use crate::prelude::*;
use bishop::prelude::*;

#[derive(Default)]
pub struct CameraManager {
    /// The game camera that is fed to the renderer.
    pub active: GameCamera,
    /// A Vec<(Entity, RoomCamera) for the currently active room.
    room_cameras: Vec<(Entity, RoomCamera)>,
    /// The id of the room we are currently tracking or `None`.
    current_room: Option<RoomId>,
    /// The stored previous position of the active game camera.
    pub previous_position: Option<Vec2>,
    runtime_follow_enabled: bool,
    runtime_override_active: bool,
}

impl CameraManager {
    /// Initialise with the player's starting room.
    pub fn new<C: BishopContext>(
        ctx: &mut C,
        ecs: &Ecs,
        room_id: RoomId,
        player_pos: Vec2,
        grid_size: f32,
    ) -> Self {
        let room_cameras = get_room_cameras(ecs, room_id);
        let (mut active_camera, _) =
            Self::find_best_camera_for_room(ecs, &room_cameras, player_pos)
                .expect("Room must contain at least one camera.");

        active_camera.camera.render_target = Some(game_render_target(ctx, grid_size));

        Self {
            active: active_camera,
            room_cameras,
            current_room: Some(room_id),
            previous_position: None,
            runtime_follow_enabled: true,
            runtime_override_active: false,
        }
    }

    /// Picks the best camera and update it if necessary.
    pub fn update_active<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        ecs: &Ecs,
        room: &Room,
        grid_size: f32,
    ) {
        // If the player moved to another room get the new cameras
        if self.current_room != Some(room.id) {
            self.current_room = Some(room.id);
            self.room_cameras = get_room_cameras(ecs, self.current_room.unwrap());
        }

        // Pick the best camera
        let player_pos = ecs
            .get_player_transform()
            .map(|t| t.position)
            .unwrap_or_default();

        if let Some((mut best_cam, mode)) =
            Self::find_best_camera_for_room(ecs, &self.room_cameras, player_pos)
        {
            // Prevent interpolation with the previous camera.
            // Only create a render target when the active camera actually changes.
            if best_cam.id != self.active.id {
                best_cam.camera.render_target = Some(game_render_target(ctx, grid_size));
                self.active = best_cam;
                self.previous_position = Some(self.active.camera.target);
            }

            // Apply follow if needed
            if self.runtime_follow_enabled && let CameraMode::Follow(restriction) = mode {
                self.apply_follow(&restriction, player_pos);
            }
        }
    }

    /// Returns whether runtime follow updates are currently enabled.
    pub fn follow_is_enabled(&self) -> bool {
        self.runtime_follow_enabled
    }

    /// Returns whether runtime camera override is currently active.
    pub fn runtime_override_is_active(&self) -> bool {
        self.runtime_override_active
    }

    /// Enables or suppresses runtime follow updates for the active camera.
    pub fn set_follow_enabled(&mut self, enabled: bool) {
        self.runtime_follow_enabled = enabled;
        self.runtime_override_active = true;
    }

    /// Applies a runtime pan delta to the active camera target.
    pub fn apply_runtime_pan_delta(&mut self, delta: Vec2) {
        self.active.camera.target += delta;
        self.runtime_override_active = true;
    }

    /// Applies a runtime zoom delta to the active camera.
    pub fn apply_runtime_zoom_delta(&mut self, delta: f32) {
        let next = self.active.camera.zoom + Vec2::splat(delta);
        self.active.camera.zoom = Vec2::new(next.x.max(f32::MIN_POSITIVE), next.y.max(f32::MIN_POSITIVE));
        self.runtime_override_active = true;
    }

    /// Clears transient runtime camera overrides after a control run ends.
    pub fn clear_runtime_overrides(&mut self) {
        self.runtime_follow_enabled = true;
        self.runtime_override_active = false;
    }

    /// Finds the most suitable camera for a given room and player position.
    pub fn find_best_camera_for_room(
        ecs: &Ecs,
        room_cameras: &[(Entity, RoomCamera)],
        player_pos: Vec2,
    ) -> Option<(GameCamera, CameraMode)> {
        // Keep track of the camera with the smallest distance to the player
        let mut closest: Option<(f32, GameCamera, CameraMode)> = None;

        for &(entity, ref cam) in room_cameras.iter() {
            let game_cam = room_to_game_camera(ecs, &entity, cam, player_pos);
            match cam.camera_mode {
                CameraMode::Fixed => {
                    if Self::point_in_camera_view(&game_cam, player_pos) {
                        return Some((game_cam, cam.camera_mode));
                    }
                }
                CameraMode::Follow(_) => {
                    return Some((game_cam, cam.camera_mode));
                }
            }
            // Squared distance between the camera centre and the player
            let dx = game_cam.camera.target.x - player_pos.x;
            let dy = game_cam.camera.target.y - player_pos.y;
            let dist_sq = dx * dx + dy * dy;

            // Update the closest so far
            match closest {
                Some((best_dist, _, _)) if dist_sq >= best_dist => {}
                _ => closest = Some((dist_sq, game_cam, cam.camera_mode)),
            }
        }
        // Return the closest as a fallback
        closest.map(|(_, cam, mode)| (cam, mode))
    }

    /// Checks whether `point` lies inside the rectangular view of `cam`.
    fn point_in_camera_view(cam: &GameCamera, point: Vec2) -> bool {
        let half_w = 1.0 / cam.camera.zoom.x;
        let half_h = 1.0 / cam.camera.zoom.y;
        let left = cam.camera.target.x - half_w;
        let right = cam.camera.target.x + half_w;
        let top = cam.camera.target.y - half_h;
        let bottom = cam.camera.target.y + half_h;
        point.x >= left && point.x <= right && point.y >= top && point.y <= bottom
    }

    /// Moves the active camera according to the follow restriction.
    fn apply_follow(&mut self, restriction: &FollowRestriction, player_pos: Vec2) {
        match restriction {
            FollowRestriction::Free => {
                self.active.camera.target = player_pos;
            }
            FollowRestriction::ClampX => {
                self.active.camera.target.y = player_pos.y;
            }
            FollowRestriction::ClampY => {
                self.active.camera.target.x = player_pos.x;
            }
        }
    }

    /// Returns the interpolated camera target for rendering.
    pub fn interpolated_target(&self, alpha: f32) -> Vec2 {
        let prev = self.previous_position.unwrap_or(self.active.camera.target);
        lerp_rounded(prev, self.active.camera.target, alpha)
    }
}
