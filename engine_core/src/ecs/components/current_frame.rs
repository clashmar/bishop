use crate::animation::ClipId;
use crate::ecs::SpriteId;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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

/// Serializable snapshot form of `CurrentFrame` runtime data.
#[serde_as]
#[derive(Clone, Deserialize, Serialize)]
pub struct CurrentFrameSnapshot {
    pub clip_id: ClipId,
    pub col: usize,
    pub row: usize,
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub offset: Vec2,
    pub sprite_id: SpriteId,
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub frame_size: Vec2,
    pub flip_x: bool,
}

impl CurrentFrame {
    /// Returns true if a texture has been assigned to this frame.
    pub fn has_valid_asset(&self) -> bool {
        self.sprite_id.0 != 0
    }
}

impl CurrentFrameSnapshot {
    /// Returns true if a texture has been assigned to this frame snapshot.
    pub fn has_valid_asset(&self) -> bool {
        self.sprite_id.0 != 0
    }
}

impl From<CurrentFrame> for CurrentFrameSnapshot {
    fn from(value: CurrentFrame) -> Self {
        let CurrentFrame {
            clip_id,
            col,
            row,
            offset,
            sprite_id,
            frame_size,
            flip_x,
        } = value;

        Self {
            clip_id,
            col,
            row,
            offset,
            sprite_id,
            frame_size,
            flip_x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_valid_asset_false_when_sprite_id_is_zero() {
        let frame = CurrentFrame::default();
        assert!(!frame.has_valid_asset());
    }

    #[test]
    fn has_valid_asset_true_when_sprite_id_is_nonzero() {
        let mut frame = CurrentFrame::default();
        frame.sprite_id = SpriteId(1);
        assert!(frame.has_valid_asset());
    }

    #[test]
    fn snapshot_has_valid_asset_false_when_sprite_id_is_zero() {
        let frame = CurrentFrameSnapshot {
            clip_id: ClipId::Idle,
            col: 0,
            row: 0,
            offset: Vec2::ZERO,
            sprite_id: SpriteId(0),
            frame_size: Vec2::ZERO,
            flip_x: false,
        };
        assert!(!frame.has_valid_asset());
    }

    #[test]
    fn snapshot_has_valid_asset_true_when_sprite_id_is_nonzero() {
        let frame = CurrentFrameSnapshot {
            clip_id: ClipId::Idle,
            col: 0,
            row: 0,
            offset: Vec2::ZERO,
            sprite_id: SpriteId(1),
            frame_size: Vec2::ZERO,
            flip_x: false,
        };
        assert!(frame.has_valid_asset());
    }
}
