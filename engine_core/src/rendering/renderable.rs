use bishop::prelude::*;
use crate::assets::SpriteManager;
use crate::ecs::{Pivot, SpriteId};

/// Parameters passed to a component's [`Renderable::draw`] method.
pub struct EntityDrawParams {
    /// Entity world position (not pivot-adjusted).
    pub pos: Vec2,
    pub pivot: Pivot,
    pub grid_size: f32,
    pub color: Color,
}

/// Trait for visual components that can draw themselves.
pub trait Renderable {
    /// Returns the pixel dimensions, or `None` if the asset is unavailable.
    fn dimensions(&self, sprite_manager: &SpriteManager) -> Option<Vec2>;
    /// Draws this component. Returns `true` if drawn, `false` if the asset is missing.
    fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
        sprite_manager: &mut SpriteManager,
        params: &EntityDrawParams,
    ) -> bool;
}

pub(crate) fn draw_emission_mask<C: BishopContext>(
    ctx: &mut C,
    sprite_manager: &mut SpriteManager,
    sprite_id: SpriteId,
    source: Rect,
    destination: Rect,
    flip_x: bool,
) -> bool {
    if ![
        source.x,
        source.y,
        source.w,
        source.h,
        destination.x,
        destination.y,
        destination.w,
        destination.h,
    ]
    .into_iter()
    .all(f32::is_finite)
        || source.w <= 0.0
        || source.h <= 0.0
        || destination.w <= 0.0
        || destination.h <= 0.0
        || sprite_id.0 == 0
    {
        return false;
    }

    let _ = sprite_manager.get_texture_from_id(ctx, sprite_id);
    if sprite_manager.texture_size(sprite_id).is_none() {
        return false;
    }
    let texture = sprite_manager.get_texture_from_id(ctx, sprite_id);
    ctx.draw_texture_ex(
        texture,
        destination.x,
        destination.y,
        Color::WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(destination.w, destination.h)),
            source: Some(source),
            flip_x,
            ..Default::default()
        },
    );
    true
}
