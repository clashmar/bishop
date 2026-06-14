use super::PropertyModule;
use crate::gui::widgets::tag_select::TagSelect;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::scripting::event_tags::event_tag::EventTag;
use widgets::*;
use ::widgets::constants::layout;

const INPUT_ROW_H: f32 = layout::DEFAULT_FIELD_HEIGHT;
const CHIP_ROW_H: f32 = layout::WIDGET_SPACING + 20.0;
const TOP_PADDING: f32 = 8.0;

/// Generic tags editor for any target type that exposes `&mut Vec<EventTag>`.
pub struct TagsPropertyModule<T> {
    tag_select: TagSelect,
    input_id: WidgetId,
    accessor: fn(&mut T) -> &mut Vec<EventTag>,
    tag_count: usize,
}

impl<T> TagsPropertyModule<T> {
    pub fn new(accessor: fn(&mut T) -> &mut Vec<EventTag>) -> Self {
        Self {
            tag_select: TagSelect::new(),
            input_id: WidgetId::default(),
            accessor,
            tag_count: 0,
        }
    }
}

impl<T: 'static> PropertyModule<T> for TagsPropertyModule<T> {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        target: &mut T,
        _game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) {
        let tags = (self.accessor)(target);
        self.tag_count = tags.len();

        let mut existing = insp_ctx.event_tags.clone();
        for tag in tags.iter() {
            if let EventTag::Custom(name) = tag {
                if !existing.iter().any(|existing_name| existing_name == name) {
                    existing.push(name.clone());
                }
            }
        }
        existing.sort_by_key(|name| name.to_ascii_lowercase());
        existing.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        self.tag_select
            .show(ctx, self.input_id, Rect::new(rect.x, rect.y + TOP_PADDING, rect.w, rect.h - TOP_PADDING), tags, &existing);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let content_h = INPUT_ROW_H + if self.tag_count > 0 { CHIP_ROW_H } else { 0.0 };
        InspectorBodyLayout::new()
            .top_padding(TOP_PADDING)
            .block(content_h)
    }

    fn title(&self) -> &str {
        "Tags"
    }
}
