use super::{PickerState, hsv_to_rgb};
use crate::{BishopContext, Color, Panel, Rect};

const PICKER_WIDTH: f32 = 224.0;
const PICKER_MARGIN: f32 = 8.0;
const SV_SIZE: f32 = 176.0;
const SV_COLUMNS: usize = 32;
const SV_ROWS: usize = 32;
const HUE_SEGMENTS: usize = 32;
const OPACITY_SEGMENTS: usize = 32;
const CONTROL_GAP: f32 = 8.0;
const CONTROL_VERTICAL_PADDING: f32 = 10.0;
const HUE_WIDTH: f32 = 18.0;
const OPACITY_HEIGHT: f32 = 18.0;

#[derive(Clone, Copy)]
pub(super) struct PickerLayout {
    pub(super) popup: Rect,
    pub(super) saturation_value: Rect,
    pub(super) hue: Rect,
    pub(super) opacity: Option<Rect>,
}

pub(super) fn picker_layout<C: BishopContext>(
    ctx: &C,
    swatch: Rect,
    opacity_enabled: bool,
) -> PickerLayout {
    let controls_height = SV_SIZE
        + if opacity_enabled {
            CONTROL_GAP + OPACITY_HEIGHT
        } else {
            0.0
        };
    let height = controls_height + CONTROL_VERTICAL_PADDING * 2.0;
    let width = PICKER_WIDTH;
    let x = swatch.x.clamp(0.0, (ctx.screen_width() - width).max(0.0));
    let below = swatch.y + swatch.h + PICKER_MARGIN;
    let y = if below + height <= ctx.screen_height() {
        below
    } else {
        (swatch.y - height - PICKER_MARGIN).max(0.0)
    };
    let popup = Rect::new(x, y, width, height);
    let controls_width = SV_SIZE + CONTROL_GAP + HUE_WIDTH;
    let controls_x = x + (width - controls_width) / 2.0;
    let saturation_value = Rect::new(
        controls_x,
        y + CONTROL_VERTICAL_PADDING,
        SV_SIZE,
        SV_SIZE,
    );
    let hue = Rect::new(
        saturation_value.x + SV_SIZE + CONTROL_GAP,
        saturation_value.y,
        HUE_WIDTH,
        SV_SIZE,
    );
    let opacity = opacity_enabled.then(|| {
        Rect::new(
            saturation_value.x,
            saturation_value.y + SV_SIZE + CONTROL_GAP,
            SV_SIZE + HUE_WIDTH + CONTROL_GAP,
            OPACITY_HEIGHT,
        )
    });

    PickerLayout {
        popup,
        saturation_value,
        hue,
        opacity,
    }
}

pub(super) fn draw_picker<C: BishopContext>(ctx: &mut C, picker: &PickerState) {
    let layout = picker.layout;
    let popup = layout.popup;
    Panel::new(popup).show(ctx);
    draw_saturation_value(ctx, layout.saturation_value, picker.hue);
    draw_hue(ctx, layout.hue);
    if let Some(opacity) = layout.opacity {
        draw_opacity(ctx, opacity, picker.color);
    }

    let saturation_marker_x = layout.saturation_value.x + picker.saturation * SV_SIZE;
    let value_marker_y = layout.saturation_value.y + (1.0 - picker.value) * SV_SIZE;
    ctx.draw_rectangle_lines(
        saturation_marker_x - 4.0,
        value_marker_y - 4.0,
        8.0,
        8.0,
        2.0,
        Color::WHITE,
    );
    let hue_marker_y = layout.hue.y + picker.hue / 360.0 * layout.hue.h;
    ctx.draw_line(
        layout.hue.x - 2.0,
        hue_marker_y,
        layout.hue.x + layout.hue.w + 2.0,
        hue_marker_y,
        2.0,
        Color::WHITE,
    );
    if let Some(opacity) = layout.opacity {
        let alpha_marker_x = opacity.x + picker.alpha * opacity.w;
        ctx.draw_rectangle_lines(
            alpha_marker_x - 2.0,
            opacity.y - 2.0,
            4.0,
            opacity.h + 4.0,
            2.0,
            Color::WHITE,
        );
    }
}

fn draw_saturation_value<C: BishopContext>(ctx: &mut C, rect: Rect, hue: f32) {
    let hue_color = hsv_to_rgb(hue, 1.0, 1.0, 1.0);
    let column_width = rect.w / SV_COLUMNS as f32;
    let row_height = rect.h / SV_ROWS as f32;
    for column in 0..SV_COLUMNS {
        let saturation = column as f32 / (SV_COLUMNS - 1) as f32;
        let color = Color::new(
            1.0 + (hue_color.r - 1.0) * saturation,
            1.0 + (hue_color.g - 1.0) * saturation,
            1.0 + (hue_color.b - 1.0) * saturation,
            1.0,
        );
        let x = rect.x + column as f32 * column_width;
        ctx.draw_rectangle(x, rect.y, column_width + 1.0, rect.h, color);
    }
    for row in 0..SV_ROWS {
        let darkness = row as f32 / (SV_ROWS - 1) as f32;
        let y = rect.y + row as f32 * row_height;
        ctx.draw_rectangle(
            rect.x,
            y,
            rect.w,
            row_height + 1.0,
            Color::new(0.0, 0.0, 0.0, darkness),
        );
    }
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::WHITE);
}

fn draw_hue<C: BishopContext>(ctx: &mut C, rect: Rect) {
    let segment_height = rect.h / HUE_SEGMENTS as f32;
    for segment in 0..HUE_SEGMENTS {
        let hue = segment as f32 / HUE_SEGMENTS as f32 * 360.0;
        let y = rect.y + segment as f32 * segment_height;
        ctx.draw_rectangle(
            rect.x,
            y,
            rect.w,
            segment_height + 1.0,
            hsv_to_rgb(hue, 1.0, 1.0, 1.0),
        );
    }
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::WHITE);
}

fn draw_opacity<C: BishopContext>(ctx: &mut C, rect: Rect, color: Color) {
    const CHECKER_SIZE: f32 = 6.0;
    let columns = (rect.w / CHECKER_SIZE).ceil() as usize;
    let rows = (rect.h / CHECKER_SIZE).ceil() as usize;
    for row in 0..rows {
        for column in 0..columns {
            let fill = if (row + column) % 2 == 0 {
                Color::new(0.75, 0.75, 0.75, 1.0)
            } else {
                Color::new(0.35, 0.35, 0.35, 1.0)
            };
            ctx.draw_rectangle(
                rect.x + column as f32 * CHECKER_SIZE,
                rect.y + row as f32 * CHECKER_SIZE,
                CHECKER_SIZE,
                CHECKER_SIZE,
                fill,
            );
        }
    }
    for segment in 0..OPACITY_SEGMENTS {
        let alpha = segment as f32 / (OPACITY_SEGMENTS - 1) as f32;
        let x = rect.x + segment as f32 * rect.w / OPACITY_SEGMENTS as f32;
        let width = rect.w / OPACITY_SEGMENTS as f32 + 1.0;
        ctx.draw_rectangle(
            x,
            rect.y,
            width,
            rect.h,
            Color::new(color.r, color.g, color.b, alpha),
        );
    }
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::WHITE);
}
