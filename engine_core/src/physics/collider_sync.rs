use crate::ecs::{ColliderData, ColliderShape, Pivot};
use bishop::prelude::{Rect, Vec2, vec2};

/// Converts frame-local rectangular bounds into AABB collider data.
pub fn collider_data_from_frame_bounds(
    frame_size: Vec2,
    bounds: Rect,
    pivot: Pivot,
) -> ColliderData {
    let collider_size = vec2(bounds.w, bounds.h);
    let pivot_offset = pivot.as_normalized();
    let frame_anchor = vec2(frame_size.x * pivot_offset.x, frame_size.y * pivot_offset.y);
    let collider_anchor = vec2(
        collider_size.x * pivot_offset.x,
        collider_size.y * pivot_offset.y,
    );

    ColliderData {
        shape: ColliderShape::Aabb {
            width: bounds.w,
            height: bounds.h,
        },
        offset: vec2(bounds.x, bounds.y) - frame_anchor + collider_anchor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb_size(collider_data: ColliderData) -> (f32, f32) {
        match collider_data.shape {
            ColliderShape::Aabb { width, height } => (width, height),
            other => panic!("expected AABB collider data, got {other:?}"),
        }
    }

    #[test]
    fn frame_bounds_to_aabb_data_respects_bottom_center_pivot() {
        let result = collider_data_from_frame_bounds(
            vec2(100.0, 80.0),
            Rect::new(10.0, 20.0, 30.0, 40.0),
            Pivot::BottomCenter,
        );

        assert_eq!(aabb_size(result), (30.0, 40.0));
        assert_eq!(result.offset, vec2(-25.0, -20.0));
    }
}
