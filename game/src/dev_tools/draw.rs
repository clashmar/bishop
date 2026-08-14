use bishop::prelude::*;
use engine_core::ecs::{Collider, SubPixel, Transform};
use engine_core::rendering::{
    draw_collider,
    outline_thickness,
    visual_position,
    ENTITY_OUTLINE_SCALE,
};
use crate::engine::game_instance::GameInstance;
use super::DevTools;

/// Draw collider outlines for all entities when dev tools collider visibility is on.
pub fn draw_colliders<C: BishopContext>(
    ctx: &mut C,
    game_instance: &GameInstance,
    dev_tools: &DevTools,
    camera: &Camera2D,
) {
    if !dev_tools.colliders_visible {
        return;
    }

    ctx.set_camera(camera);

    let ecs = &game_instance.game.ecs;
    let collider_store = ecs.get_store::<Collider>();
    let transform_store = ecs.get_store::<Transform>();
    let sub_pixel_store = ecs.get_store::<SubPixel>();
    let grid_size = game_instance.game.current_world().grid_size;
    let thickness = outline_thickness(grid_size) * ENTITY_OUTLINE_SCALE;
    let color = Color::PINK;

    for (entity, collider) in collider_store.data.iter() {
        if let Some(transform) = transform_store.get(*entity) {
            draw_collider(
                ctx,
                visual_position(transform.position, sub_pixel_store.get(*entity)),
                collider,
                transform.pivot,
                color,
                thickness,
            );
        }
    }

    ctx.set_default_camera();
}
