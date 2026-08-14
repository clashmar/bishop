//! Drawing primitives and textures.

mod params;

pub use params::*;

use crate::types::{Color, Texture2D, Vec2};

/// Core drawing operations for 2D primitives.
pub trait Draw {
    /// Draws a filled rectangle.
    fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color);

    /// Draws a rectangle outline.
    fn draw_rectangle_lines(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color,
    );

    /// Draws a line between two points.
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color);

    /// Draws a filled circle.
    fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: Color);

    /// Draws a circle outline.
    fn draw_circle_lines(&mut self, x: f32, y: f32, radius: f32, thickness: f32, color: Color);

    /// Draws an arc outline.
    fn draw_arc_lines(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        thickness: f32,
        color: Color,
    );

    /// Draws a filled triangle.
    fn draw_triangle(&mut self, v1: Vec2, v2: Vec2, v3: Vec2, color: Color);

    /// Clears the screen with the specified color.
    fn clear_background(&mut self, color: Color);

    /// Draws a texture at the specified position.
    fn draw_texture(&mut self, texture: &Texture2D, x: f32, y: f32, color: Color);

    /// Draws a texture with extended parameters.
    fn draw_texture_ex(
        &mut self,
        texture: &Texture2D,
        x: f32,
        y: f32,
        color: Color,
        params: DrawTextureParams,
    );

    /// Restricts subsequent rendering to the given rectangle.
    ///
    /// Must be paired with [`pop_clip_rect`](Self::pop_clip_rect).
    fn push_clip_rect(&mut self, rect: crate::types::Rect);

    /// Removes the active clip rectangle set by [`push_clip_rect`](Self::pop_clip_rect).
    fn pop_clip_rect(&mut self);
}

/// Draws an arrow from one point to another.
pub fn draw_arrow(
    ctx: &mut impl Draw,
    from: Vec2,
    to: Vec2,
    color: Color,
    head_length: f32,
    head_half_width: f32,
) {
    let delta = to - from;
    if delta.length_squared() == 0.0 {
        return;
    }

    let direction = delta.normalize();
    let tip = to;
    let shaft_end = to - direction * head_length;
    let normal = Vec2::new(-direction.y, direction.x) * head_half_width;

    ctx.draw_line(from.x, from.y, shaft_end.x, shaft_end.y, 2.0, color);
    ctx.draw_triangle(tip, shaft_end + normal, shaft_end - normal, color);
}
