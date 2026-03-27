use super::{EDIT_SECTION_SPACING, SECTION_GAP, SPACING};
use engine_core::prelude::InspectorBodyLayout;

pub(super) fn body_height(
    has_groups: bool,
    rename_active: bool,
    preset_actions_visible: bool,
    sounds_len: usize,
) -> f32 {
    let mut layout = InspectorBodyLayout::new().rows(1, SPACING);

    if rename_active {
        layout = layout.gap(SPACING).rows(1, SPACING);
    }

    if !has_groups {
        return layout.height();
    }

    if preset_actions_visible {
        layout = layout.gap(SPACING).rows(1, SPACING);
    }

    layout
        .gap(SECTION_GAP)
        .rows(sounds_len + 5, EDIT_SECTION_SPACING)
        .height()
}
