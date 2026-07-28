pub mod collapsible;
pub mod game;
pub mod room;
pub mod tags_module;
pub mod tilemap;
pub mod world;

use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
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
        insp_ctx: &InspectorContext,
    );

    /// Optional host action produced during the last `draw` call.
    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        None
    }

    /// Whether this room-only module requested the interior-zone tool toggle.
    fn take_toggle_room_zone_tool(&mut self) -> bool {
        false
    }

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
}
