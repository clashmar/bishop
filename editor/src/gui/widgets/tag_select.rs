use bishop::prelude::*;
use engine_core::scripting::event_tags::event_tag::{builtin_event_tags, EventTag};
use engine_core::scripting::lua_constants::lua_event_tag;
use engine_core::ui::constants::layout;
use engine_core::ui::{MultiSelect, MultiSelectDelta, WidgetId};

pub struct TagSelect;

impl TagSelect {
    pub fn new() -> Self {
        Self
    }

    pub fn show<C: BishopContext>(
        &self,
        ctx: &mut C,
        id: WidgetId,
        rect: Rect,
        tags: &mut Vec<EventTag>,
        event_tags: &[String],
    ) -> Option<MultiSelectDelta<EventTag>> {
        let input_rect = Rect::new(rect.x, rect.y, rect.w, layout::DEFAULT_FIELD_HEIGHT);

        let mut options: Vec<EventTag> = builtin_event_tags().collect();
        for tag in tags.iter() {
            if !options.contains(tag) {
                options.push(tag.clone());
            }
        }
        for name in event_tags {
            let custom = EventTag::Custom(name.clone());
            if !options.contains(&custom) {
                options.push(custom);
            }
        }
        options.sort_by_key(|tag| tag.display_name().to_ascii_lowercase());

        let delta = MultiSelect::new(id, input_rect, "Add tags...", &options, tags, |t| match t {
            EventTag::Autosave => lua_event_tag::AUTOSAVE.to_string(),
            EventTag::Custom(name) => name.clone(),
        })
        .filterable(true)
        .create_new(|s| {
            if let Some(existing) = options
                .iter()
                .find(|tag| tag.display_name().eq_ignore_ascii_case(s))
            {
                existing.clone()
            } else {
                EventTag::Custom(s.to_string())
            }
        })
        .show(ctx);

        if delta.is_some() {
            tags.sort_by_key(|tag| tag.display_name().to_ascii_lowercase());
        }

        delta
    }
}
