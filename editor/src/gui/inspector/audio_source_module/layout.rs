use crate::gui::widgets::audio_source_module_core::{EDIT_SECTION_SPACING, SECTION_GAP, SPACING};
use engine_core::ecs::inspector::InspectorBodyLayout;

pub(crate) fn body_layout(
    has_groups: bool,
    rename_active: bool,
    preset_actions_visible: bool,
    has_fade_duration: bool,
    sounds_len: usize,
) -> InspectorBodyLayout {
    let mut layout = InspectorBodyLayout::new().rows(1, SPACING);

    if rename_active {
        layout = layout.gap(SPACING).rows(1, SPACING);
    }

    if !has_groups {
        return layout;
    }

    if preset_actions_visible {
        layout = layout.gap(SPACING).rows(1, SPACING);
    }

    let mut fixed_rows = 7;
    if has_fade_duration {
        fixed_rows += 1;
    }

    layout
        .gap(SECTION_GAP)
        .rows(sounds_len + fixed_rows, EDIT_SECTION_SPACING)
}
