pub mod collapsible;
pub mod room;

use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;

/// A collapsible module in an editor mode properties panel.
pub trait PropertyModule<T> {
    /// Show this module for the given target?
    fn visible(&self, _target: &T, _game_ctx: &GameCtxMut) -> bool {
        true
    }

    /// Draw the module body (inside the collapsible wrapper, when expanded).
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        target: &mut T,
        game_ctx: &mut GameCtxMut,
    );

    /// Layout describing expanded body height.
    fn body_layout(&self) -> InspectorBodyLayout;

    /// Height when expanded.
    fn height(&self) -> f32 {
        self.body_layout().height()
    }

    /// Title shown in the collapsible header. Defaults to the struct name.
    fn title(&self) -> &str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap_or("Module")
    }

    /// Whether any input in this module is actively being edited.
    fn was_input_active(&self) -> bool {
        false
    }
}
