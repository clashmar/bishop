use crate::worlds::room::{Room, RoomId};
use bishop::prelude::*;

/// Create a `Room` for testing, with an optional explicit id.
#[cfg(test)]
pub fn make_room(id: Option<usize>, x: f32, y: f32, w: f32, h: f32) -> Room {
    Room {
        id: RoomId(id.unwrap_or(0)),
        position: Vec2::new(x, y),
        size: Vec2::new(w, h),
        ..Default::default()
    }
}
