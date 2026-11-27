use bishop::{Text, prelude::{Rect, WgpuContext}};
use widgets::{
    InputCommit, NumberInput, Slider, SliderState, Widget, WidgetId,
    constants::{colors, layout},
};

pub(crate) const FIELD_HEIGHT: f32 = 30.0;
pub(crate) const FIELD_GAP: f32 = layout::WIDGET_SPACING;
pub(crate) const LABEL_WIDTH: f32 = 100.0;
pub(crate) const LABEL_Y_OFFSET: f32 = 20.0;
pub(crate) const NUMERIC_INPUT_WIDTH: f32 = 72.0;
pub(crate) const NUMERIC_SLIDER_GAP: f32 = 8.0;
pub(crate) const TOP_PADDING: f32 = layout::WIDGET_SPACING;
pub(crate) const BOTTOM_GUTTER: f32 = 10.0;

pub(crate) fn draw_label(ctx: &mut WgpuContext, label: &str, rect: Rect, y: f32) {
    ctx.draw_text(
        label,
        rect.x,
        y + LABEL_Y_OFFSET,
        layout::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
}

pub(crate) fn field_rect(rect: Rect, y: f32) -> Rect {
    field_rect_with_label_width(rect, y, LABEL_WIDTH)
}

pub(crate) fn field_rect_with_label_width(rect: Rect, y: f32, label_width: f32) -> Rect {
    Rect::new(rect.x + label_width, y, rect.w - label_width, FIELD_HEIGHT)
}

pub(crate) fn numeric_slider_rects(rect: Rect, y: f32) -> (Rect, Rect) {
    let field = field_rect(rect, y);
    let input_width = NUMERIC_INPUT_WIDTH.min((field.w - NUMERIC_SLIDER_GAP).max(0.0));
    let input = Rect::new(field.x, y, input_width, FIELD_HEIGHT);
    let slider_x = input.x + input.w + NUMERIC_SLIDER_GAP;
    let slider = Rect::new(
        slider_x,
        y,
        (field.x + field.w - slider_x).max(0.0),
        FIELD_HEIGHT,
    );
    (input, slider)
}

pub(crate) fn draw_float_slider(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    y: f32,
    input_id: WidgetId,
    slider_id: WidgetId,
    current: f32,
    max: f32,
    decimal_places: u32,
) -> (f32, bool) {
    let current = normalized_slider_value(current, 0.0, max, decimal_places);
    let (input_rect, slider_rect) = numeric_slider_rects(rect, y);
    let (typed, input_commit) = NumberInput::new(input_id, input_rect, current)
        .min(0.0)
        .max(max)
        .blocked(blocked)
        .show(ctx);
    let (slider_value, slider_state) = Slider::new(slider_id, slider_rect, 0.0, max, current)
        .blocked(blocked)
        .show(ctx);
    let input_changed = input_active(input_commit);
    let slider_changed = !matches!(slider_state, SliderState::Unchanged);
    let value = if slider_changed {
        slider_value
    } else if input_changed {
        typed
    } else {
        current
    };
    (
        normalized_slider_value(value, 0.0, max, decimal_places),
        input_changed || slider_changed,
    )
}

pub(crate) fn draw_integer_slider(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    y: f32,
    input_id: WidgetId,
    slider_id: WidgetId,
    current: i32,
    min: i32,
    max: i32,
) -> (i32, bool) {
    let current = current.clamp(min, max);
    let (input_rect, slider_rect) = numeric_slider_rects(rect, y);
    let (typed, input_commit) = NumberInput::new(input_id, input_rect, current)
        .min(min)
        .max(max)
        .blocked(blocked)
        .show(ctx);
    let (slider_value, slider_state) = Slider::new(
        slider_id,
        slider_rect,
        min as f32,
        max as f32,
        current as f32,
    )
    .blocked(blocked)
    .show(ctx);
    let input_changed = input_active(input_commit);
    let slider_changed = !matches!(slider_state, SliderState::Unchanged);
    let value = if slider_changed {
        slider_value.round() as i32
    } else if input_changed {
        typed
    } else {
        current
    };
    (value.clamp(min, max), input_changed || slider_changed)
}

pub(crate) fn input_active(commit: InputCommit) -> bool {
    matches!(commit, InputCommit::Previewing | InputCommit::Committed)
}

pub(crate) fn round_to_dp(value: f32, decimal_places: u32) -> f32 {
    let factor = 10_f32.powi(decimal_places as i32);
    let rounded = (value * factor).round() / factor;
    if (rounded - rounded.round()).abs() < 1e-6 {
        rounded.round()
    } else {
        rounded
    }
}

pub(crate) fn normalized_slider_value(
    value: f32,
    min: f32,
    max: f32,
    decimal_places: u32,
) -> f32 {
    let value = if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    };
    round_to_dp(value, decimal_places)
}

#[cfg(test)]
mod tests {
    use super::{numeric_slider_rects, normalized_slider_value};
    use bishop::prelude::Rect;

    #[test]
    fn numeric_slider_rects_keep_a_compact_input_and_use_remaining_width() {
        let (input, slider) = numeric_slider_rects(Rect::new(10.0, 20.0, 300.0, 30.0), 45.0);

        assert_eq!(input, Rect::new(110.0, 45.0, 72.0, 30.0));
        assert_eq!(slider, Rect::new(190.0, 45.0, 120.0, 30.0));
    }

    #[test]
    fn normalized_slider_value_clamps_and_rounds_to_the_requested_precision() {
        assert_eq!(normalized_slider_value(1.236, 0.0, 4.0, 2), 1.24);
        assert_eq!(normalized_slider_value(5.0, 0.0, 4.0, 2), 4.0);
    }
}
