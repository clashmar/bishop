//! 2D camera for controlling the viewport.

use crate::{types::Rect, wgpu::BishopRenderTarget};
use glam::{vec2, vec3, Mat4, Vec2};

/// 2D camera for controlling the viewport.
#[derive(Clone, Debug)]
pub struct Camera2D {
    /// The point in world space the camera is looking at.
    pub target: Vec2,
    /// Zoom level (higher = more zoomed in).
    pub zoom: Vec2,
    /// Rotation in radians.
    pub rotation: f32,
    /// Offset from the target in screen space.
    pub offset: Vec2,
    pub render_target: Option<BishopRenderTarget>,
    pub viewport: Option<(i32, i32, i32, i32)>,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            zoom: Vec2::ONE,
            rotation: 0.0,
            offset: Vec2::ZERO,
            render_target: None,
            viewport: None,
        }
    }
}

impl Camera2D {
    /// Creates a new camera with the given target and zoom.
    pub fn new(target: Vec2, zoom: Vec2) -> Self {
        Self {
            target,
            zoom,
            ..Default::default()
        }
    }

    /// Converts a world position to screen coordinates.
    pub fn world_to_screen(&self, world_pos: Vec2, screen_w: f32, screen_h: f32) -> Vec2 {
        let dims = self
            .viewport()
            .map(|(vx, vy, vw, vh)| Rect {
                x: vx as f32,
                y: screen_h - (vy + vh) as f32,
                w: vw as f32,
                h: vh as f32,
            })
            .unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                w: screen_w,
                h: screen_h,
            });

        let mat = self.matrix();
        let transformed = mat.transform_point3(vec3(world_pos.x, world_pos.y, 0.0));

        vec2(
            (transformed.x + 1.0) / 2.0 * dims.w + dims.x,
            (1.0 - transformed.y) / 2.0 * dims.h + dims.y,
        )
    }

    /// Returns the world space position for a 2d camera screen space position.
    pub fn screen_to_world(&self, point: Vec2, screen_w: f32, screen_h: f32) -> Vec2 {
        let dims = self
            .viewport()
            .map(|(vx, vy, vw, vh)| Rect {
                x: vx as f32,
                y: screen_h - (vy + vh) as f32,
                w: vw as f32,
                h: vh as f32,
            })
            .unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                w: screen_w,
                h: screen_h,
            });

        let point = vec2(
            (point.x - dims.x) / dims.w * 2. - 1.,
            1. - (point.y - dims.y) / dims.h * 2.,
        );
        let inv_mat = self.matrix().inverse();
        let transform = inv_mat.transform_point3(vec3(point.x, point.y, 0.));

        vec2(transform.x, transform.y)
    }

    fn matrix(&self) -> Mat4 {
        let mat_origin = Mat4::from_translation(vec3(-self.target.x, -self.target.y, 0.0));
        let mat_rotation = Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), self.rotation.to_radians());
        let invert_y = -1.0;
        let mat_scale = Mat4::from_scale(vec3(self.zoom.x, self.zoom.y * invert_y, 1.0));
        let mat_translation = Mat4::from_translation(vec3(self.offset.x, self.offset.y, 0.0));

        mat_translation * ((mat_scale * mat_rotation) * mat_origin)
    }

    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        self.viewport
    }
}
