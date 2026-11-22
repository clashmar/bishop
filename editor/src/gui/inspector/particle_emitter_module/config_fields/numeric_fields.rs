use std::fmt::{Debug, Display};
use std::str::FromStr;

use bishop::prelude::{Rect, Text, Vec2, WgpuContext};
use engine_core::ecs::components::ParticleEmitter;
use widgets::constants::{colors, layout};
use widgets::{InputCommit, WidgetId};

use super::super::{ParticleEmitterModule, VisibleControls};
use super::{
    AREA_LABEL, BURST_COUNT_LABEL, EMISSION_RATE_LABEL, FIELD_GAP, LIFETIME_LABEL,
    LINE_LENGTH_LABEL, OPACITY_LABEL, PARTICLE_LIMIT_LABEL, ROW_HEIGHT, SCALE_LABEL,
    SEED_LABEL, VELOCITY_X_LABEL, VELOCITY_Y_LABEL, bounded_number_input, draw_label,
    field_rect, pair_rect, record_commit, update_config,
};

pub(super) fn draw_base(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    y: &mut f32,
) {
    let area_size = emitter.config.area_size;
    let (area, area_x_commit, area_y_commit) = draw_vec2(
        ctx,
        blocked,
        row_rect(rect, *y),
        AREA_LABEL,
        [module.widget_ids.area_x, module.widget_ids.area_y],
        area_size,
        (Some(0.0), None),
    );
    record_commit(module, area_x_commit);
    record_commit(module, area_y_commit);
    if area != area_size {
        update_config(module, emitter, |config| config.area_size = area);
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    draw_label(ctx, LIFETIME_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let (lifetime_min, min_commit) = bounded_number_input(
        module.widget_ids.lifetime_min,
        pair_rect(ctx, rect, *y, 0),
        emitter.config.lifetime_min,
        blocked,
        Some(f32::MIN_POSITIVE),
        Some(emitter.config.lifetime_max),
    )
    .show(ctx);
    let (lifetime_max, max_commit) = bounded_number_input(
        module.widget_ids.lifetime_max,
        pair_rect(ctx, rect, *y, 1),
        emitter.config.lifetime_max,
        blocked,
        Some(emitter.config.lifetime_min),
        None,
    )
    .show(ctx);
    record_commit(module, min_commit);
    record_commit(module, max_commit);
    if lifetime_min != emitter.config.lifetime_min || lifetime_max != emitter.config.lifetime_max {
        update_config(module, emitter, |config| {
            config.lifetime_min = lifetime_min;
            config.lifetime_max = lifetime_max;
        });
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    let velocity_x_min = emitter.config.velocity_min.x;
    let velocity_x_max = emitter.config.velocity_max.x;
    let (updated_velocity_x_min, updated_velocity_x_max, min_commit, max_commit) = draw_pair(
        ctx,
        blocked,
        row_rect(rect, *y),
        VELOCITY_X_LABEL,
        [module.widget_ids.velocity_min_x, module.widget_ids.velocity_max_x],
        [velocity_x_min, velocity_x_max],
        (None, None),
    );
    record_commit(module, min_commit);
    record_commit(module, max_commit);
    if updated_velocity_x_min != velocity_x_min || updated_velocity_x_max != velocity_x_max {
        update_config(module, emitter, |config| {
            config.velocity_min.x = updated_velocity_x_min;
            config.velocity_max.x = updated_velocity_x_max;
        });
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    let velocity_y_min = emitter.config.velocity_min.y;
    let velocity_y_max = emitter.config.velocity_max.y;
    let (updated_velocity_y_min, updated_velocity_y_max, min_commit, max_commit) = draw_pair(
        ctx,
        blocked,
        row_rect(rect, *y),
        VELOCITY_Y_LABEL,
        [module.widget_ids.velocity_min_y, module.widget_ids.velocity_max_y],
        [velocity_y_min, velocity_y_max],
        (None, None),
    );
    record_commit(module, min_commit);
    record_commit(module, max_commit);
    if updated_velocity_y_min != velocity_y_min || updated_velocity_y_max != velocity_y_max {
        update_config(module, emitter, |config| {
            config.velocity_min.y = updated_velocity_y_min;
            config.velocity_max.y = updated_velocity_y_max;
        });
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    let (scale_start, scale_end, start_commit, end_commit) = draw_pair(
        ctx,
        blocked,
        row_rect(rect, *y),
        SCALE_LABEL,
        [module.widget_ids.scale_start, module.widget_ids.scale_end],
        [emitter.config.scale_start, emitter.config.scale_end],
        (Some(0.0), None),
    );
    record_commit(module, start_commit);
    record_commit(module, end_commit);
    if scale_start != emitter.config.scale_start || scale_end != emitter.config.scale_end {
        update_config(module, emitter, |config| {
            config.scale_start = scale_start;
            config.scale_end = scale_end;
        });
    }
    *y += ROW_HEIGHT + FIELD_GAP;

}

pub(super) fn draw_opacity(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    y: &mut f32,
) {
    let (opacity_start, opacity_end, start_commit, end_commit) = draw_pair(
        ctx,
        blocked,
        row_rect(rect, *y),
        OPACITY_LABEL,
        [module.widget_ids.opacity_start, module.widget_ids.opacity_end],
        [emitter.config.opacity_start, emitter.config.opacity_end],
        (Some(0.0), Some(1.0)),
    );
    record_commit(module, start_commit);
    record_commit(module, end_commit);
    if opacity_start != emitter.config.opacity_start || opacity_end != emitter.config.opacity_end {
        update_config(module, emitter, |config| {
            config.opacity_start = opacity_start;
            config.opacity_end = opacity_end;
        });
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

pub(super) fn draw_emission(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    controls: VisibleControls,
    y: &mut f32,
) {
    if controls.line_length {
        let (value, commit) = draw_scalar(
            ctx,
            blocked,
            row_rect(rect, *y),
            LINE_LENGTH_LABEL,
            module.widget_ids.line_length,
            emitter.config.line_length,
            (Some(1), None),
        );
        record_commit(module, commit);
        if value != emitter.config.line_length {
            update_config(module, emitter, |config| config.line_length = value);
        }
        *y += ROW_HEIGHT + FIELD_GAP;
    }
    if controls.burst_count {
        let (value, commit) = draw_scalar(
            ctx,
            blocked,
            row_rect(rect, *y),
            BURST_COUNT_LABEL,
            module.widget_ids.burst_count,
            emitter.config.burst_count,
            (Some(1), None),
        );
        record_commit(module, commit);
        if value != emitter.config.burst_count {
            update_config(module, emitter, |config| config.burst_count = value);
        }
        *y += ROW_HEIGHT + FIELD_GAP;
    }
    if controls.emission_rate {
        let (value, commit) = draw_scalar(
            ctx,
            blocked,
            row_rect(rect, *y),
            EMISSION_RATE_LABEL,
            module.widget_ids.emission_rate,
            emitter.config.emission_rate,
            (Some(f32::MIN_POSITIVE), None),
        );
        record_commit(module, commit);
        if value != emitter.config.emission_rate {
            update_config(module, emitter, |config| config.emission_rate = value);
        }
        *y += ROW_HEIGHT + FIELD_GAP;
    }

    let (max_particles, commit) = draw_scalar(
        ctx,
        blocked,
        row_rect(rect, *y),
        PARTICLE_LIMIT_LABEL,
        module.widget_ids.max_particles,
        emitter.config.max_particles,
        (Some(1), None),
    );
    record_commit(module, commit);
    if max_particles != emitter.config.max_particles {
        update_config(module, emitter, |config| config.max_particles = max_particles);
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    let (seed, commit) = draw_scalar(
        ctx,
        blocked,
        row_rect(rect, *y),
        SEED_LABEL,
        module.widget_ids.seed,
        emitter.config.seed,
        (None, None),
    );
    record_commit(module, commit);
    if seed != emitter.config.seed {
        update_config(module, emitter, |config| config.seed = seed);
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

pub(super) fn draw_validation_issues(
    module: &ParticleEmitterModule,
    ctx: &mut WgpuContext,
    rect: Rect,
    mut y: f32,
) {
    let issues = module.validation_issues.borrow();
    let mut height = 0.0;
    for (index, issue) in issues.iter().enumerate() {
        if index > 0 {
            y += FIELD_GAP;
            height += FIELD_GAP;
        }
        let message_height = ctx.draw_text_wrapped(
            issue.message(),
            rect.x,
            y,
            layout::FIELD_TEXT_SIZE_16 - 2.0,
            colors::DEFAULT_DANGER_COLOR,
            rect.w,
        );
        y += message_height;
        height += message_height;
    }
    module.validation_height.set(height);
}

fn row_rect(rect: Rect, y: f32) -> Rect {
    Rect::new(rect.x, y, rect.w, rect.h)
}

fn draw_vec2(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    label: &str,
    ids: [WidgetId; 2],
    value: Vec2,
    bounds: (Option<f32>, Option<f32>),
) -> (Vec2, InputCommit, InputCommit) {
    draw_label(ctx, label, rect.x, rect.y, colors::DEFAULT_TEXT_COLOR);
    let (x, x_commit) = bounded_number_input(
        ids[0],
        pair_rect(ctx, rect, rect.y, 0),
        value.x,
        blocked,
        bounds.0,
        bounds.1,
    )
    .show(ctx);
    let (y, y_commit) = bounded_number_input(
        ids[1],
        pair_rect(ctx, rect, rect.y, 1),
        value.y,
        blocked,
        bounds.0,
        bounds.1,
    )
    .show(ctx);
    (Vec2::new(x, y), x_commit, y_commit)
}

fn draw_pair(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    label: &str,
    ids: [WidgetId; 2],
    values: [f32; 2],
    bounds: (Option<f32>, Option<f32>),
) -> (f32, f32, InputCommit, InputCommit) {
    let (values, first_commit, second_commit) = draw_vec2(
        ctx,
        blocked,
        rect,
        label,
        ids,
        Vec2::new(values[0], values[1]),
        bounds,
    );
    (values.x, values.y, first_commit, second_commit)
}

fn draw_scalar<T>(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    label: &str,
    id: WidgetId,
    value: T,
    bounds: (Option<T>, Option<T>),
) -> (T, InputCommit)
where
    T: Copy + PartialEq + PartialOrd + FromStr + Display + Default,
    <T as FromStr>::Err: Debug,
{
    draw_label(ctx, label, rect.x, rect.y, colors::DEFAULT_TEXT_COLOR);
    let (value, commit) = bounded_number_input(
        id,
        field_rect(ctx, rect, rect.y),
        value,
        blocked,
        bounds.0,
        bounds.1,
    )
    .show(ctx);
    (value, commit)
}
