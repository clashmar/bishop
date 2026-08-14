use crate::assets::sprite_manager::SpriteManager;
use crate::inspector_module;
use crate::rendering::helpers::pivot_adjusted_position;
use crate::rendering::renderable::{EntityDrawParams, Renderable};
use bishop::prelude::*;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Opaque handle that the asset manager gives out. Default/Unset is 0.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub struct SpriteId(pub usize);

#[ecs_component]
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Sprite {
    /// Reference to the texture stored by the AssetManager.
    pub sprite: SpriteId,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            sprite: SpriteId(0),
        }
    }
}

inspector_module!(Sprite);

impl Sprite {
    /// Returns true if a texture has been assigned to this sprite.
    pub fn has_valid_asset(&self) -> bool {
        self.sprite.0 != 0
    }

    fn resolved_draw_size(texture_size: Option<(f32, f32)>, grid_size: f32) -> Vec2 {
        texture_size
            .map(|(w, h)| vec2(w, h))
            .unwrap_or(Vec2::splat(grid_size))
    }
}

impl Renderable for Sprite {
    fn dimensions(&self, sprite_manager: &SpriteManager) -> Option<Vec2> {
        sprite_manager
            .texture_size(self.sprite)
            .map(|(w, h)| vec2(w, h))
    }

    fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
        sprite_manager: &mut SpriteManager,
        params: &EntityDrawParams,
    ) -> bool {
        if !self.has_valid_asset() {
            return false;
        }

        let _ = sprite_manager.get_texture_from_id(ctx, self.sprite);
        let size = Self::resolved_draw_size(
            sprite_manager.texture_size(self.sprite),
            params.grid_size,
        );
        let tex = sprite_manager.get_texture_from_id(ctx, self.sprite);
        let draw_base = pivot_adjusted_position(params.pos, size, params.pivot);
        ctx.draw_texture_ex(
            tex,
            draw_base.x,
            draw_base.y,
            params.color,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_valid_asset_false_when_sprite_id_is_zero() {
        let sprite = Sprite::default();
        assert!(!sprite.has_valid_asset());
    }

    #[test]
    fn has_valid_asset_true_when_sprite_id_is_nonzero() {
        let sprite = Sprite {
            sprite: SpriteId(42),
        };
        assert!(sprite.has_valid_asset());
    }

    #[test]
    fn resolved_draw_size_uses_grid_size_when_texture_is_missing() {
        assert_eq!(Sprite::resolved_draw_size(None, 16.0), vec2(16.0, 16.0));
    }

    #[test]
    fn resolved_draw_size_uses_texture_size_when_available() {
        assert_eq!(
            Sprite::resolved_draw_size(Some((24.0, 12.0)), 16.0),
            vec2(24.0, 12.0),
        );
    }
}
