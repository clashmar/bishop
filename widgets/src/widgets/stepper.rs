use crate::*;

/// Draws a stepper widget that allows selecting from a list of predefined values.
///
/// Returns the selected value from the steps array.
pub fn gui_stepper<C: BishopContext>(
    ctx: &mut C,
    rect: impl Into<Rect>,
    label: &str,
    steps: &[f32],
    current: f32,
    blocked: bool,
) -> f32 {
    let rect = rect.into();
    let mut idx = steps
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (*a - current)
                .abs()
                .partial_cmp(&(*b - current).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    const Y_OFFSET: f32 = 15.0;

    let label = format!("{}:", label);
    let label_width = measure_text_ui(ctx, &label, FIELD_TEXT_SIZE_16).width;

    let btn_w = FIELD_TEXT_SIZE_16 * 1.2;
    let val_w = measure_text_ui(ctx, "3.0", FIELD_TEXT_SIZE_16).width + WIDGET_SPACING + 5.0;

    draw_text_ui(
        ctx,
        &label,
        rect.x,
        rect.y,
        FIELD_TEXT_SIZE_16,
        FIELD_TEXT_COLOR,
    );

    let val_rect = Rect::new(
        rect.x + label_width + WIDGET_SPACING,
        rect.y - Y_OFFSET,
        val_w,
        rect.h,
    );

    ctx.draw_rectangle_lines(
        val_rect.x,
        val_rect.y - 7.5,
        val_rect.w,
        btn_w + 15.0,
        2.,
        OUTLINE_COLOR,
    );

    let txt = format!("{:.1}", steps[idx]);
    draw_text_ui(
        ctx,
        &txt,
        val_rect.x + 7.5,
        val_rect.y + 17.5,
        FIELD_TEXT_SIZE_16,
        FIELD_TEXT_COLOR,
    );

    let decrease_rect = Rect::new(
        val_rect.x + val_w + WIDGET_SPACING,
        rect.y - Y_OFFSET,
        btn_w,
        btn_w,
    );

    if Button::new(decrease_rect, "-").blocked(blocked).show(ctx) && idx > 0 {
        idx -= 1;
    }

    let increase_rect = Rect::new(
        decrease_rect.x + btn_w + WIDGET_SPACING,
        rect.y - Y_OFFSET,
        btn_w,
        btn_w,
    );
    if Button::new(increase_rect, "+").blocked(blocked).show(ctx) && idx + 1 < steps.len() {
        idx += 1;
    }

    steps[idx]
}
