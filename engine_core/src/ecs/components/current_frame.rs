use crate::animation::ClipId;
use crate::ecs::SpriteId;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Current frame data for rendering animated entities.
#[ecs_component]
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct CurrentFrame {
    #[serde(skip)]
    pub clip_id: ClipId,
    #[serde(skip)]
    pub col: usize,
    #[serde(skip)]
    pub row: usize,
    #[serde(skip)]
    pub offset: Vec2,
    #[serde(skip)]
    pub sprite_id: SpriteId,
    #[serde(skip)]
    pub frame_size: Vec2,
    /// Whether to flip the sprite horizontally when rendering.
    #[serde(skip)]
    pub flip_x: bool,
}
