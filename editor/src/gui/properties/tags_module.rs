use super::PropertyModule;
use crate::gui::widgets::tag_select::TagSelect;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::scripting::event_tags::event_tag::EventTag;
use widgets::*;
use ::widgets::constants::layout;

/// Generic tags editor for any target type that exposes `&mut Vec<EventTag>`.
pub struct TagsPropertyModule<T> {
    tag_select: TagSelect,
    input_id: WidgetId,
    accessor: fn(&mut T) -> &mut Vec<EventTag>,
}

impl<T> TagsPropertyModule<T> {
    pub fn new(accessor: fn(&mut T) -> &mut Vec<EventTag>) -> Self {
        Self {
            tag_select: TagSelect::new(),
            input_id: WidgetId::default(),
            accessor,
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
            .show(ctx, self.input_id, rect, tags, &existing);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().rows(1, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Tags"
    }
}
