use super::draw::{Vertex, WebRenderer};
use crate::types::{Color, Vec2};

impl WebRenderer {
    pub(crate) fn draw_colored_rectangle(
        &self, x: f32, y: f32, w: f32, h: f32, color: Color,
    ) {
        let positions = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        self.draw_quad(positions, [[0.0, 0.0]; 4], self.white.inner_web().handle(), color);
    }

    pub(crate) fn draw_rectangle_outline(
        &self, x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color,
    ) {
        if thickness <= 0.0 {
            return;
        }
        self.draw_colored_rectangle(x, y, w, thickness, color);
        self.draw_colored_rectangle(x, y + h - thickness, w, thickness, color);
        self.draw_colored_rectangle(x, y + thickness, thickness, h - thickness * 2.0, color);
        self.draw_colored_rectangle(
            x + w - thickness, y + thickness, thickness, h - thickness * 2.0, color,
        );
    }

    pub(crate) fn draw_colored_line(
        &self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color,
    ) {
        if thickness <= 0.0 {
            return;
        }
        let delta = Vec2::new(x2 - x1, y2 - y1);
        if delta.length_squared() == 0.0 {
            self.draw_colored_rectangle(
                x1 - thickness * 0.5, y1 - thickness * 0.5, thickness, thickness, color,
            );
            return;
        }
        let normal = Vec2::new(-delta.y, delta.x).normalize() * (thickness * 0.5);
        let start = Vec2::new(x1, y1);
        let end = Vec2::new(x2, y2);
        let positions = [
            (start + normal).to_array(),
            (end + normal).to_array(),
            (end - normal).to_array(),
            (start - normal).to_array(),
        ];
        self.draw_quad(positions, [[0.0, 0.0]; 4], self.white.inner_web().handle(), color);
    }

    pub(crate) fn draw_colored_triangle(&self, v1: Vec2, v2: Vec2, v3: Vec2, color: Color) {
        let vertices = [
            Vertex::solid(v1.to_array()),
            Vertex::solid(v2.to_array()),
            Vertex::solid(v3.to_array()),
        ];
        self.draw_triangles(&vertices, self.white.inner_web().handle(), color);
    }

    pub(crate) fn draw_colored_circle(&self, x: f32, y: f32, radius: f32, color: Color) {
        if radius <= 0.0 {
            return;
        }
        const SEGMENTS: usize = 32;
        let center = Vec2::new(x, y);
        for index in 0..SEGMENTS {
            let start = index as f32 * std::f32::consts::TAU / SEGMENTS as f32;
            let end = (index + 1) as f32 * std::f32::consts::TAU / SEGMENTS as f32;
            let first = center + Vec2::new(start.cos(), start.sin()) * radius;
            let second = center + Vec2::new(end.cos(), end.sin()) * radius;
            self.draw_colored_triangle(center, first, second, color);
        }
    }

    pub(crate) fn draw_circle_outline(
        &self, x: f32, y: f32, radius: f32, thickness: f32, color: Color,
    ) {
        self.draw_arc(
            Vec2::new(x, y), radius, 0.0, std::f32::consts::TAU, thickness, color,
        );
    }

    pub(crate) fn draw_arc(
        &self, center: Vec2, radius: f32, start_angle: f32,
        end_angle: f32, thickness: f32, color: Color,
    ) {
        if radius <= 0.0 || thickness <= 0.0 {
            return;
        }
        const SEGMENTS: usize = 16;
        let mut previous = center + Vec2::new(start_angle.cos(), start_angle.sin()) * radius;
        for index in 1..=SEGMENTS {
            let t = index as f32 / SEGMENTS as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            let current = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            self.draw_colored_line(
                previous.x, previous.y, current.x, current.y, thickness, color,
            );
            previous = current;
        }
    }
}
