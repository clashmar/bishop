use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;
use widgets::InputCommit;

/// Draw a label + text input row. Returns the committed value.
pub(crate) fn draw_labeled_text_input(
    ctx: &mut WgpuContext,
    rect: Rect,
    label: &str,
    value: &str,
    widget_id: WidgetId,
) -> (String, InputCommit) {
    let label_measure = measure_text(ctx, label, layout::DEFAULT_FONT_SIZE_16);
    ctx.draw_text(
        label,
        rect.x,
        rect.y + 20.0,
        layout::DEFAULT_FONT_SIZE_16,
        Color::WHITE,
    );
    let input_rect = Rect::new(
        rect.x + label_measure.width + layout::WIDGET_SPACING,
        rect.y,
        rect.w - label_measure.width - layout::WIDGET_SPACING,
        layout::DEFAULT_FIELD_HEIGHT,
    );
    TextInput::new(widget_id, input_rect, value).show(ctx)
}

/// Emits the new name only on committed edits that actually changed the value.
pub(crate) fn committed_name_change(current: &str, edited: &str, commit: InputCommit) -> Option<String> {
    matches!(commit, InputCommit::Committed)
        .then_some(())
        .and_then(|_| (edited != current).then(|| edited.to_string()))
}
