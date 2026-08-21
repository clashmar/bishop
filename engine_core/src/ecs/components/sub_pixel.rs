use super::transform::update_entity_position;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Accumulated sub-pixel remainder for pixel-perfect physics.
#[ecs_component]
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct SubPixel {
    #[serde(skip)]
    pub x: f32,
    #[serde(skip)]
    pub y: f32,
}

/// Returns a position with its stored sub-pixel remainder applied.
pub fn true_position(position: Vec2, sub_pixel: SubPixel) -> Vec2 {
    position + Vec2::new(sub_pixel.x, sub_pixel.y)
}

/// Returns the quantized position and stored remainder after applying a delta.
pub fn quantize_motion(position: Vec2, sub_pixel: SubPixel, delta: Vec2) -> (Vec2, SubPixel) {
    let new_true_pos = true_position(position, sub_pixel) + delta;
    let new_int_pos = new_true_pos.round();

    (
        new_int_pos,
        SubPixel {
            x: new_true_pos.x - new_int_pos.x,
            y: new_true_pos.y - new_int_pos.y,
        },
    )
}

/// Writes a quantized position and sub-pixel remainder back to an entity.
pub fn apply_quantized_state(
    ecs: &mut Ecs,
    entity: Entity,
    position: Vec2,
    sub_pixel: SubPixel,
) {
    update_entity_position(ecs, entity, position);
    store_sub_pixel(ecs, entity, sub_pixel);
}

/// Applies a delta, then writes the quantized result back to an entity.
pub fn apply_quantized_delta(
    ecs: &mut Ecs,
    entity: Entity,
    position: Vec2,
    sub_pixel: SubPixel,
    delta: Vec2,
) {
    let (new_position, new_sub_pixel) = quantize_motion(position, sub_pixel, delta);
    apply_quantized_state(ecs, entity, new_position, new_sub_pixel);
}

fn store_sub_pixel(ecs: &mut Ecs, entity: Entity, sub_pixel: SubPixel) {
    if let Some(existing) = ecs.get_mut::<SubPixel>(entity) {
        *existing = sub_pixel;
        return;
    }

    if sub_pixel.x != 0.0 || sub_pixel.y != 0.0 {
        ecs.add_component_to_entity(entity, sub_pixel);
    }
}
