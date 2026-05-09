// editor/src/editor/camera_controller.rs
use bishop::prelude::*;
use engine_core::prelude::*;

pub const ZOOM_STEP_PERCENT: f32 = 0.5;
pub const PIXEL_ZOOM_PERCENT_PER_DELTA: f32 = 0.01;
pub const MAX_PIXEL_ZOOM_PERCENT: f32 = 0.2;
pub const MIN_ZOOM: f32 = 0.0005;
pub const MAX_ZOOM: f32 = 0.1;

pub struct EditorCameraController;

impl EditorCameraController {
    /// Call this once per frame from any editor that owns a `Camera2D`.
    pub fn update(ctx: &WgpuContext, camera: &mut Camera2D) {
        // Pan
        if ctx.is_mouse_button_down(MouseButton::Middle) || ctx.is_key_down(KeyCode::Space) {
            let delta = ctx.mouse_delta_position();
            let delta_vec = vec2(delta.0, delta.1);
            let screen_size = vec2(ctx.screen_width(), ctx.screen_height());
            camera.target -= delta_vec * 2.0 / (camera.zoom * screen_size);
        }

        // Zoom (mouse wheel) - zoom towards mouse cursor
        let scroll = ctx.mouse_wheel().1;
        if let Some(zoom_multiplier) =
            Self::zoom_multiplier_for_scroll(scroll, ctx.mouse_wheel_kind())
        {
            let wheel_kind = ctx.mouse_wheel_kind().unwrap_or(MouseWheelKind::Line);
            let mouse_screen = ctx.mouse_position();
            let mouse_screen = vec2(mouse_screen.0, mouse_screen.1);

            // Get world position under mouse before zoom
            let screen_w = ctx.screen_width();
            let screen_h = ctx.screen_height();
            let world_before = camera.screen_to_world(mouse_screen, screen_w, screen_h);

            // Apply zoom
            let mut scalar = Self::current_scalar(ctx, camera);
            scalar *= zoom_multiplier;
            scalar = scalar.clamp(MIN_ZOOM, MAX_ZOOM);
            Self::apply_aspect_with_snap(ctx, camera, scalar, wheel_kind == MouseWheelKind::Line);

            // Get world position under mouse after zoom
            let world_after = camera.screen_to_world(mouse_screen, screen_w, screen_h);

            // Adjust target so the original world position stays under the mouse
            camera.target += world_before - world_after;
        } else {
            let scalar = Self::current_scalar(ctx, camera);
            Self::apply_aspect_with_snap(
                ctx,
                camera,
                scalar,
                Self::should_snap_current_scalar(ctx.screen_height(), scalar),
            );
        }
    }

    fn zoom_multiplier_for_scroll(scroll: f32, wheel_kind: Option<MouseWheelKind>) -> Option<f32> {
        if scroll.abs() <= f32::EPSILON {
            return None;
        }

        let zoom_percent = match wheel_kind.unwrap_or(MouseWheelKind::Line) {
            MouseWheelKind::Line => ZOOM_STEP_PERCENT,
            MouseWheelKind::Pixel => {
                (scroll.abs() * PIXEL_ZOOM_PERCENT_PER_DELTA).min(MAX_PIXEL_ZOOM_PERCENT)
            }
        };

        Some(1.0 + scroll.signum() * zoom_percent)
    }

    /// Returns the scalar zoom that would be used for a square window.
    pub fn scalar_zoom(ctx: &WgpuContext, camera: &Camera2D) -> f32 {
        Self::current_scalar(ctx, camera)
    }

    // Retrieve the *scalar* zoom that represents the true world‑unit
    // size, regardless of the current aspect ratio.
    pub fn current_scalar(ctx: &WgpuContext, camera: &Camera2D) -> f32 {
        let aspect = ctx.screen_width() / ctx.screen_height();
        if aspect > 1.0 {
            // Y holds the scalar
            camera.zoom.y
        } else {
            // X holds the scalar
            camera.zoom.x
        }
    }

    // Turn a scalar zoom into a non‑uniform pair that keeps world
    // units square for the current aspect ratio, snapped to integer pixel ratios.
    pub fn apply_aspect(ctx: &WgpuContext, camera: &mut Camera2D, scalar_zoom: f32) {
        Self::apply_aspect_with_snap(ctx, camera, scalar_zoom, true);
    }

    fn apply_aspect_with_snap(
        ctx: &WgpuContext,
        camera: &mut Camera2D,
        scalar_zoom: f32,
        snap_to_integer_scale: bool,
    ) {
        let win_w = ctx.screen_width();
        let win_h = ctx.screen_height();

        camera.zoom = Self::zoom_for_scalar(win_w, win_h, scalar_zoom, snap_to_integer_scale);
    }

    fn zoom_for_scalar(
        win_w: f32,
        win_h: f32,
        scalar_zoom: f32,
        snap_to_integer_scale: bool,
    ) -> Vec2 {
        let resolved_scalar = if snap_to_integer_scale {
            // Snap to integer pixel scale based on the smaller dimension
            // scale = screen_size * zoom / 2.0, so zoom = 2.0 * scale / screen_size
            let current_scale = (win_h * scalar_zoom / 2.0).round().max(1.0);
            2.0 * current_scale / win_h
        } else {
            scalar_zoom
        };

        let aspect = win_w / win_h;
        let (zoom_x, zoom_y) = if aspect > 1.0 {
            (resolved_scalar / aspect, resolved_scalar)
        } else {
            (resolved_scalar, resolved_scalar * aspect)
        };
        vec2(zoom_x, zoom_y)
    }

    fn should_snap_current_scalar(win_h: f32, scalar_zoom: f32) -> bool {
        let current_scale = win_h * scalar_zoom / 2.0;
        (current_scale - current_scale.round()).abs() <= 0.001
    }

    /// Returns a camera centered on a room.
    pub fn camera_for_room(
        ctx: &WgpuContext,
        room_size: Vec2,
        room_position: Vec2,
        grid_size: f32,
    ) -> Camera2D {
        let max_dim_px = (room_size * grid_size).max_element() / 2.0;
        let scalar = editor_zoom_factor(grid_size) / max_dim_px;

        let mut camera = Camera2D {
            target: (room_position + (room_size * grid_size) / 2.0),
            ..Default::default()
        };
        Self::apply_aspect(ctx, &mut camera, scalar);
        camera
    }

    /// Centers a `Camera2D` on a world position with a default zoom for the prefab editor.
    pub fn reset_prefab_editor_camera(
        ctx: &WgpuContext,
        camera: &mut Camera2D,
        entity_position: Vec2,
        grid_size: f32,
    ) {
        let view_tiles = 4.0_f32;
        let max_dim_px = view_tiles * grid_size / 1.5;
        let scalar = editor_zoom_factor(grid_size) / max_dim_px;
        camera.target = entity_position;
        Self::apply_aspect(ctx, camera, scalar);
    }

    /// Reset a `Camera2D` so that the whole room fits the screen.
    pub fn reset_room_editor_camera(
        ctx: &WgpuContext,
        camera: &mut Camera2D,
        room: &Room,
        grid_size: f32,
    ) {
        let map_size = vec2(
            room.current_variant().tilemap.width as f32,
            room.current_variant().tilemap.height as f32,
        );
        *camera = Self::camera_for_room(ctx, map_size, room.position, grid_size);
    }

    /// Returns a zoom vector that makes the whole `size` fit the screen,
    /// respecting the current aspect ratio (higher = more zoom)
    pub fn zoom_for_size(ctx: &WgpuContext, size: Vec2, zoom_factor: f32, grid_size: f32) -> Vec2 {
        let max_dim_px = size.max_element() / zoom_factor;
        let scalar = editor_zoom_factor(grid_size) / max_dim_px;
        let mut temp = Camera2D {
            zoom: vec2(scalar, scalar),
            ..Default::default()
        };
        EditorCameraController::apply_aspect(ctx, &mut temp, scalar);
        temp.zoom
    }
}
