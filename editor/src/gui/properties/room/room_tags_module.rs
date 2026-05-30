use super::super::PropertyModule;
use super::draw_labeled_text_input;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::prelude::*;
use engine_core::worlds::room::Room;

pub struct RoomTagsModule {
    input_id: WidgetId,
}

impl RoomTagsModule {
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
        }
    }
}

impl Default for RoomTagsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<Room> for RoomTagsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        _game_ctx: &mut GameCtxMut,
    ) {
        let tags_value = room.tags.join(", ");
        let (new_val, commit) =
            draw_labeled_text_input(ctx, rect, "Tags:", &tags_value, self.input_id);

        if !matches!(commit, InputCommit::Unchanged) {
            room.tags.clear();
            for part in new_val.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    room.tags.push(trimmed.to_string());
                }
            }
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, widgets::constants::layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Tags"
    }
}
