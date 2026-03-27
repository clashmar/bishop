// editor/src/world/coord.rs
use bishop::prelude::*;

/// Convert the current mouse position (screen pixels) to world
/// coordinates using the supplied camera.
pub fn mouse_world_pos(ctx: &WgpuContext, camera: &Camera2D) -> Vec2 {
    let (x, y) = ctx.mouse_position();
    camera.screen_to_world(vec2(x, y), ctx.screen_width(), ctx.screen_height())
}

/// Snap an world‑space point to the integer grid that the
/// editor works with.
pub fn snap_to_grid(pos: Vec2) -> Vec2 {
    vec2(pos.x.floor(), pos.y.floor())
}

/// Return the grid cell (integer coordinates) that the mouse is
/// hovering over.
pub fn mouse_world_grid(ctx: &WgpuContext, camera: &Camera2D, grid_size: f32) -> Vec2 {
    let world = mouse_world_pos(ctx, camera);
    (world / grid_size).floor()
}

/// Turn a world‑space `Vec2` into screen coordinates using the current camera.
pub fn world_to_screen(ctx: &WgpuContext, camera: &Camera2D, world_pos: Vec2) -> Vec2 {
    camera.world_to_screen(world_pos, ctx.screen_width(), ctx.screen_height())
}

/// Check if a room overlaps with existing rooms.
/// pos and size should be in tile coordinates.
/// other_bounds contains (position in pixels, size in tiles).
pub fn overlaps_existing_rooms(
    pos: Vec2,
    size: Vec2,
    other_bounds: &[(Vec2, Vec2)],
    grid_size: f32,
) -> bool {
    let a_min = pos;
    let a_max = pos + size;

    other_bounds.iter().any(|(b_pos, b_size)| {
        let b_min = *b_pos / grid_size;
        let b_max = b_min + *b_size;

        a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
    })
}
