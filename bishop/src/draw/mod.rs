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

    /// Removes the active clip rectangle set by [`push_clip_rect`](Self::push_clip_rect).
    fn pop_clip_rect(&mut self);
}
