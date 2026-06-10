//! Axis-aligned rectangle type.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Axis-aligned rectangle defined by position and size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Creates a new rectangle.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Returns true if the point is inside the rectangle.
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }

    /// Returns the center point of the rectangle.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Left (minimum x) edge.
    #[inline]
    pub const fn left(&self) -> f32 {
        self.x
    }

    /// Right (maximum x) edge.
    #[inline]
    pub const fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Top (minimum y) edge.
    #[inline]
    pub const fn top(&self) -> f32 {
        self.y
    }

    /// Bottom (maximum y) edge.
    #[inline]
    pub const fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Returns the top-left corner.
    pub fn top_left(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Returns the bottom-right corner.
    pub fn bottom_right(&self) -> Vec2 {
        Vec2::new(self.x + self.w, self.y + self.h)
    }

    /// Returns the size of the rect.
    pub const fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }

    /// Returns true if this rect overlaps another rect.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Returns the overlapping area between two rects.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min_x = self.x.max(other.x);
        let min_y = self.y.max(other.y);
        let max_x = (self.x + self.w).min(other.x + other.w);
        let max_y = (self.y + self.h).min(other.y + other.h);

        if min_x >= max_x || min_y >= max_y {
            return None;
        }

        Some(Self::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlaps_returns_true_for_intersecting_rects() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);

        assert!(a.overlaps(&b));
    }

    #[test]
    fn overlaps_returns_false_for_touching_edges() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 5.0, 5.0);

        assert!(!a.overlaps(&b));
    }

    #[test]
    fn intersection_returns_overlap_rect() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(4.0, 6.0, 10.0, 10.0);

        assert_eq!(a.intersection(&b), Some(Rect::new(4.0, 6.0, 6.0, 4.0)));
    }

    #[test]
    fn intersection_returns_none_for_touching_edges() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 5.0, 5.0);

        assert_eq!(a.intersection(&b), None);
    }
}
