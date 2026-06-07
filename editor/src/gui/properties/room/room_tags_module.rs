use super::super::PropertyModule;
use crate::gui::widgets::tag_select::TagSelect;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::scripting::event_tags::event_tag::EventTag;
use ::widgets::*;
use engine_core::worlds::room::Room;

/// Edits the tags of a room.
pub struct RoomTagsModule {
    tag_select: TagSelect,
    input_id: WidgetId,
}

impl RoomTagsModule {
    /// Creates a new room tags module.
    pub fn new() -> Self {
        Self {
            tag_select: TagSelect::new(),
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
        insp_ctx: &InspectorContext,
    ) {
        let mut existing = insp_ctx.event_tags.clone();
        for tag in &room.tags {
            if let EventTag::Custom(name) = tag {
                if !existing.iter().any(|existing_name| existing_name == name) {
                    existing.push(name.clone());
                }
            }
        }
        existing.sort_by_key(|name| name.to_ascii_lowercase());
        existing.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        self.tag_select
            .show(ctx, self.input_id, rect, &mut room.tags, &existing);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, widgets::constants::layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Tags"
    }
}
