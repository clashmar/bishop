use bishop::prelude::*;
use crate::ecs::{Collider, ColliderShape, Pivot};
use crate::rendering::pivot_adjusted_position;

/// Draw the outline of a collider at the given entity position.
pub fn draw_collider<C: BishopContext>(
    ctx: &mut C,
    entity_pos: Vec2,
    collider: &Collider,
    pivot: Pivot,
    color: Color,
    thickness: f32,
) {
    match collider.shape {
        ColliderShape::Aabb { width, height } => {
            if width <= 0.0 || height <= 0.0 {
                return;
            }
            let draw_pos = pivot_adjusted_position(
                entity_pos + collider.offset,
                vec2(width, height),
                pivot,
            );
            ctx.draw_rectangle_lines(draw_pos.x, draw_pos.y, width, height, thickness, color);
        }
        ColliderShape::Circle { radius } => {
            if radius <= 0.0 {
                return;
            }
            let size = Vec2::splat(radius * 2.0);
            let draw_pos = pivot_adjusted_position(
                entity_pos + collider.offset,
                size,
                pivot,
            );
            ctx.draw_circle_lines(
                draw_pos.x + radius,
                draw_pos.y + radius,
                radius,
                thickness,
                color,
            );
        }
        ColliderShape::Capsule { radius, height } => {
            if radius <= 0.0 {
                return;
            }
            let size = vec2(radius * 2.0, height + radius * 2.0);
            let draw_pos = pivot_adjusted_position(
                entity_pos + collider.offset,
                size,
                pivot,
            );
            draw_capsule_outline(ctx, draw_pos, radius, height, thickness, color);
        }
        ColliderShape::Point => {
            let point = entity_pos + collider.offset;
            ctx.draw_circle_lines(point.x, point.y, 2.0, thickness, color);
        }
    }
}

fn draw_capsule_outline<C: BishopContext>(
    ctx: &mut C,
    top_left: Vec2,
    radius: f32,
    height: f32,
    thickness: f32,
    color: Color,
) {
    let top_center = vec2(top_left.x + radius, top_left.y + radius);
    let bottom_center = vec2(top_left.x + radius, top_left.y + radius + height);

    ctx.draw_line(
        top_center.x - radius,
        top_center.y,
        bottom_center.x - radius,
        bottom_center.y,
        thickness,
        color,
    );
    ctx.draw_line(
        top_center.x + radius,
        top_center.y,
        bottom_center.x + radius,
        bottom_center.y,
        thickness,
        color,
    );
    ctx.draw_arc_lines(
        top_center,
        radius,
        std::f32::consts::PI,
        std::f32::consts::PI * 2.0,
        thickness,
        color,
    );
    ctx.draw_arc_lines(
        bottom_center,
        radius,
        0.0,
        std::f32::consts::PI,
        thickness,
        color,
    );
}
