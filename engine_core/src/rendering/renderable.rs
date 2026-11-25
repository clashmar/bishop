use bishop::prelude::*;
use crate::animation::AsepriteSliceBounds;
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

pub(crate) fn emission_slice_geometry(
    frame_source_origin: Vec2,
    frame_position: Vec2,
    frame_size: Vec2,
    bounds: AsepriteSliceBounds,
    flip_x: bool,
) -> (Rect, Rect) {
    let destination_x = if flip_x {
        frame_size.x - bounds.x - bounds.w
    } else {
        bounds.x
    };
    (
        Rect::new(
            frame_source_origin.x + bounds.x,
            frame_source_origin.y + bounds.y,
            bounds.w,
            bounds.h,
        ),
        Rect::new(
            frame_position.x + destination_x,
            frame_position.y + bounds.y,
            bounds.w,
            bounds.h,
        ),
    )
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
