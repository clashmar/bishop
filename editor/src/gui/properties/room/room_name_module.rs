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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_input_rect_is_vertically_centered_in_module_body() {
        let body_rect = Rect::new(10.0, 20.0, 200.0, 42.0);

        assert_eq!(
            centered_name_field_rect(body_rect),
            Rect::new(10.0, 26.0, 200.0, layout::DEFAULT_FIELD_HEIGHT)
        );
    }
}

fn centered_name_field_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        rect.y + (rect.h - layout::DEFAULT_FIELD_HEIGHT) / 2.0,
        rect.w,
        layout::DEFAULT_FIELD_HEIGHT,
    )
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
        let field_rect = centered_name_field_rect(rect);
        let (new_val, _commit) =
            draw_labeled_text_input(ctx, field_rect, "Name:", &room.name, self.input_id);
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
