use crate::prelude::*;
use bishop::prelude::*;

/// Parameters passed to a component's [`Renderable::draw`] method.
pub struct EntityDrawParams {
    /// Entity world position (not pivot-adjusted).
    pub pos: Vec2,
    pub pivot: Pivot,
    pub grid_size: f32,
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
