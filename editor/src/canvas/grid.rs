// editor/src/canvas/grid.rs
use crate::{
    app::camera_controller::{self, EditorCameraController},
    canvas::grid_shader::{GridParams, GridRenderer},
    world::world_editor::LINE_THICKNESS_MULTIPLIER,
};
use bishop::prelude::*;
use glam::vec2;

const GRID_LINE_COLOR: Color = Color::new(0.5, 0.5, 0.5, 0.2);

/// Draw a grid overlay for the editor using a shader.
pub fn draw_grid(
    ctx: &mut WgpuContext,
    grid_renderer: &GridRenderer,
    camera: &Camera2D,
    grid_size: f32,
) {
    let scalar = EditorCameraController::scalar_zoom(ctx, camera);
    if scalar < camera_controller::MIN_ZOOM * 4.0 {
        return;
    }

    let params = GridParams {
        camera_pos: camera.target,
        camera_zoom: scalar,
        viewport_size: vec2(ctx.screen_width(), ctx.screen_height()),
        grid_size,
        line_color: GRID_LINE_COLOR,
        line_thickness: LINE_THICKNESS_MULTIPLIER / 2.0,
    };

    grid_renderer.draw(ctx, &params);
}
