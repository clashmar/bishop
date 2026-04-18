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
