use bishop::prelude::*;
use engine_core::animation::ClipDef;
use engine_core::ui::measure_text;
use widgets::constants::{colors, layout};
use widgets::{Checkbox, NumberInput, Rect, Widget};

use super::{
    AnimationModule, COLON_GAP, COLUMN_GAP, INLINE_GROUP_GAP, LABEL_FONT_SIZE,
    LABEL_Y_OFFSET, NUM_FIELD_W,
};
use crate::gui::gui_constants::{CHECKBOX_SIZE, INPUT_HEIGHT};

pub(super) fn draw_frame_size_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Frame X:", "Frame Y:"];
    let (lbl_x, inp_x, lbl_y, inp_y) = layout_pair(ctx, y, rect, LABELS);

    // Render the two labels
    ctx.draw_text(
        LABELS[0],
        lbl_x.x,
        lbl_x.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_y.x,
        lbl_y.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );

    // Numeric inputs
    let (fw, _) = NumberInput::new(module.frame_x_id, inp_x, clip.frame_size.x)
        .blocked(blocked)
        .show(ctx);
    clip.frame_size.x = fw;
    let (fh, _) = NumberInput::new(module.frame_y_id, inp_y, clip.frame_size.y)
        .blocked(blocked)
        .show(ctx);
    clip.frame_size.y = fh;
}

pub(super) fn draw_spritesheet_dimension_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Cols:", "Rows:"];
    let (lbl_c, inp_c, lbl_r, inp_r) = layout_pair(ctx, y, rect, LABELS);

    ctx.draw_text(
        LABELS[0],
        lbl_c.x,
        lbl_c.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_r.x,
        lbl_r.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );

    let (c, _) = NumberInput::new(module.cols_id, inp_c, clip.cols as f32)
        .blocked(blocked)
        .show(ctx);
    clip.cols = c as usize;
    let (r, _) = NumberInput::new(module.rows_id, inp_r, clip.rows as f32)
        .blocked(blocked)
        .show(ctx);
    clip.rows = r as usize;
}

pub(super) fn draw_fps_loop_and_mirrored(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    let start_x = rect.x + layout::WIDGET_PADDING;
    let fps_label = "FPS:";
    let loop_label = "Loop:";
    let mirror_label = "Mirror:";

    let fps_label_w = measure_text(ctx, fps_label, LABEL_FONT_SIZE).width + COLON_GAP;
    let loop_label_w = measure_text(ctx, loop_label, LABEL_FONT_SIZE).width + COLON_GAP;
    let mirror_label_w = measure_text(ctx, mirror_label, LABEL_FONT_SIZE).width + COLON_GAP;

    let lbl_fps = Rect::new(start_x, y + LABEL_Y_OFFSET, fps_label_w, INPUT_HEIGHT);
    let inp_fps = Rect::new(lbl_fps.x + lbl_fps.w, y, NUM_FIELD_W, INPUT_HEIGHT);

    let loop_label_x = inp_fps.x + inp_fps.w + INLINE_GROUP_GAP;
    let lbl_loop = Rect::new(loop_label_x, y + LABEL_Y_OFFSET, loop_label_w, INPUT_HEIGHT);
    let inp_loop = Rect::new(lbl_loop.x + lbl_loop.w, y + 5.0, CHECKBOX_SIZE, CHECKBOX_SIZE);

    let mirror_label_x = inp_loop.x + inp_loop.w + INLINE_GROUP_GAP;
    let lbl_mirror = Rect::new(
        mirror_label_x,
        y + LABEL_Y_OFFSET,
        mirror_label_w,
        INPUT_HEIGHT,
    );
    let inp_mirror = Rect::new(
        lbl_mirror.x + lbl_mirror.w,
        y + 5.0,
        CHECKBOX_SIZE,
        CHECKBOX_SIZE,
    );

    ctx.draw_text(
        fps_label,
        lbl_fps.x,
        lbl_fps.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        loop_label,
        lbl_loop.x,
        lbl_loop.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        mirror_label,
        lbl_mirror.x,
        lbl_mirror.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );

    let (nfps, _) = NumberInput::new(module.fps_id, inp_fps, clip.fps)
        .blocked(blocked)
        .show(ctx);
    clip.fps = nfps;
    Checkbox::new(inp_loop, &mut clip.looping)
        .blocked(blocked)
        .show(ctx);
    Checkbox::new(inp_mirror, &mut clip.mirrored)
        .blocked(blocked)
        .show(ctx);
}

pub(super) fn draw_offset_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Offset X:", "Offset Y:"];
    let (lbl_x, inp_x, lbl_y, inp_y) = layout_pair(ctx, y, rect, LABELS);

    ctx.draw_text(
        LABELS[0],
        lbl_x.x,
        lbl_x.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_y.x,
        lbl_y.y,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );

    let (ox, _) = NumberInput::new(module.offset_x_id, inp_x, clip.offset.x)
        .blocked(blocked)
        .show(ctx);
    clip.offset.x = ox;
    let (oy, _) = NumberInput::new(module.offset_y_id, inp_y, clip.offset.y)
        .blocked(blocked)
        .show(ctx);
    clip.offset.y = oy;
}

/// Returns the two label rects and the two input rects for a horizontal pair of fields.
fn layout_pair(
    ctx: &mut WgpuContext,
    y: f32,
    rect: Rect,
    labels: [&'static str; 2],
) -> (Rect, Rect, Rect, Rect) {
    let start_x = rect.x + layout::WIDGET_PADDING;
    let available_width = rect.w - 2.0 * layout::WIDGET_PADDING;
    let column_width = (available_width - COLUMN_GAP) / 2.0;

    let width1 = measure_text(ctx, labels[0], LABEL_FONT_SIZE).width + COLON_GAP;
    let width2 = measure_text(ctx, labels[1], LABEL_FONT_SIZE).width + COLON_GAP;

    let label1 = Rect::new(
        start_x,
        y + LABEL_Y_OFFSET,
        width1.min(column_width - NUM_FIELD_W).max(0.0),
        INPUT_HEIGHT,
    );
    let input0 = Rect::new(
        start_x + column_width - NUM_FIELD_W,
        y,
        NUM_FIELD_W,
        INPUT_HEIGHT,
    );

    let second_x = start_x + column_width + COLUMN_GAP;
    let label2 = Rect::new(
        second_x,
        y + LABEL_Y_OFFSET,
        width2.min(column_width - NUM_FIELD_W).max(0.0),
        INPUT_HEIGHT,
    );
    let input1 = Rect::new(
        second_x + column_width - NUM_FIELD_W,
        y,
        NUM_FIELD_W,
        INPUT_HEIGHT,
    );

    (label1, input0, label2, input1)
}
