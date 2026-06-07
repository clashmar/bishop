use super::super::PropertyModule;
use crate::gui::text_input::draw_labeled_text_input;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use ::widgets::constants::layout;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use widgets::*;
use engine_core::worlds::room::Room;

/// Edits the name of a room.
pub struct RoomNameModule {
    input_id: WidgetId,
}

impl RoomNameModule {
    /// Creates a new room name module.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
        }
    }
}

impl Default for RoomNameModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<Room> for RoomNameModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let (new_val, _commit) =
            draw_labeled_text_input(ctx, rect, "Name:", &room.name, self.input_id);
        if new_val != room.name {
            room.name = new_val;
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Name"
    }
}
