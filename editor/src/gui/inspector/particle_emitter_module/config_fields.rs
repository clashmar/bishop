use std::fmt::{Debug, Display};
use std::str::FromStr;

use bishop::prelude::{Draw, DrawTextureParams, Rect, Vec2, WgpuContext};
use bishop::types::Color;
use engine_core::animation::ClipId;
use engine_core::assets::{AssetRegistry, SpriteManager};
use engine_core::ecs::components::{
    CurrentFrameSnapshot, ParticleEmissionMode, ParticleEmitter, ParticleEmitterConfig,
    ParticleVisual,
};
use engine_core::ecs::Entity;
use crate::editor_global::push_editor_action;
use engine_core::game::GameCtxMut;
use engine_core::storage::assets_folder;
use engine_core::ui::measure_text;
use strum_macros::Display;
use widgets::constants::{colors, layout};
use widgets::{
    Button, ColorInput, ColorInputEdit, Dropdown, InputCommit, Label, NumberInput, Widget, WidgetId,
};

use super::{ParticleEmitterModule, apply_config_candidate};

mod numeric_fields;
pub(super) mod tooltips;

pub(super) const ROW_HEIGHT: f32 = 30.0;
pub(super) const FIELD_GAP: f32 = layout::WIDGET_SPACING;
const LABEL_Y_OFFSET: f32 = 20.0;
pub(super) const MODE_LABEL: &str = "Mode:";
pub(super) const VISUAL_LABEL: &str = "Visual:";
pub(super) const AREA_LABEL: &str = "Area X / Y:";
pub(super) const LIFETIME_LABEL: &str = "Lifetime:";
pub(super) const VELOCITY_X_LABEL: &str = "X velocity:";
pub(super) const VELOCITY_Y_LABEL: &str = "Y velocity:";
pub(super) const SCALE_LABEL: &str = "Scale:";
pub(super) const TINT_LABEL: &str = "Tint:";
pub(super) const TINT_ALPHA_LABEL: &str = "Alpha:";
pub(super) const OPACITY_LABEL: &str = "Opacity:";
pub(super) const LINE_LENGTH_LABEL: &str = "Line length:";
pub(super) const SPRITE_FRAME_LABEL: &str = "Sprite:";
pub(super) const BURST_COUNT_LABEL: &str = "Burst count:";
pub(super) const EMISSION_RATE_LABEL: &str = "Rate:";
pub(super) const PARTICLE_LIMIT_LABEL: &str = "Particle limit:";
pub(super) const SEED_LABEL: &str = "Seed:";

struct SpriteAssetContext<'a> {
    asset_registry: &'a mut AssetRegistry,
    sprite_manager: &'a mut SpriteManager,
}

#[derive(Default)]
pub(super) struct WidgetIds {
    mode: WidgetId,
    visual: WidgetId,
    sprite_frame: WidgetId,
    area_x: WidgetId,
    area_y: WidgetId,
    line_length: WidgetId,
    lifetime_min: WidgetId,
    lifetime_max: WidgetId,
    velocity_min_x: WidgetId,
    velocity_min_y: WidgetId,
    velocity_max_x: WidgetId,
    velocity_max_y: WidgetId,
    scale_start: WidgetId,
    scale_end: WidgetId,
    color: WidgetId,
    color_alpha: WidgetId,
    opacity_start: WidgetId,
    opacity_end: WidgetId,
    burst_count: WidgetId,
    emission_rate: WidgetId,
    max_particles: WidgetId,
    seed: WidgetId,
}

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
enum VisualKind {
    Pixel,
    Line,
    Sprite,
}

pub(super) fn draw(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    game_ctx: &mut GameCtxMut,
    entity: Entity,
) {
    module.was_editing.set(false);
    let (ecs, asset_registry, sprite_manager) = (
        &mut *game_ctx.ecs,
        &mut *game_ctx.asset_registry,
        &mut *game_ctx.sprite_manager,
    );
    let mut sprite_assets = SpriteAssetContext {
        asset_registry,
        sprite_manager,
    };
    let Some(emitter) = ecs.get_mut::<ParticleEmitter>(entity) else {
        return;
    };
    let mut y = rect.y + FIELD_GAP;
    if module.preview_control_visible.get() {
        draw_preview_control(module, ctx, blocked, rect, emitter, &mut y);
    }
    draw_mode_field(module, ctx, blocked, rect, emitter, &mut y);
    draw_visual_field(
        module,
        ctx,
        blocked,
        rect,
        emitter,
        &mut sprite_assets,
        &mut y,
    );

    numeric_fields::draw_base(module, ctx, blocked, rect, emitter, &mut y);
    draw_color_field(module, ctx, blocked, rect, emitter, &mut y);
    numeric_fields::draw_opacity(module, ctx, blocked, rect, emitter, &mut y);

    let controls = ParticleEmitterModule::visible_controls(&emitter.config);
    module.controls.set(controls);
    if controls.sprite_frame {
        draw_sprite_frame_field(
            module,
            ctx,
            blocked,
            rect,
            emitter,
            &mut sprite_assets,
            &mut y,
        );
    }
    numeric_fields::draw_emission(module, ctx, blocked, rect, emitter, controls, &mut y);
    numeric_fields::draw_validation_issues(module, ctx, rect, y);
}

fn draw_preview_control(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &ParticleEmitter,
    y: &mut f32,
) {
    let (label, tooltip) = match emitter.config.mode {
        ParticleEmissionMode::Burst => (
            "Preview burst",
            "Emit one burst into the room preview.",
        ),
        ParticleEmissionMode::Continuous => (
            "Toggle preview",
            "Start or stop continuous emission in the room preview.",
        ),
    };
    let button_rect = Rect::new(rect.x, *y, rect.w, ROW_HEIGHT);
    if Button::new(button_rect, label)
        .tooltip(tooltip)
        .suppressed(blocked || emitter.config.validate().is_err())
        .show(ctx)
    {
        if let Some(owner) = module.issue_entity.get() {
            push_editor_action(move |editor| {
                let (room_editor, game) = (&mut editor.room_editor, &editor.game);
                room_editor.preview_particle_emitter(owner, &game.ecs);
            });
        }
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

fn draw_mode_field(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    y: &mut f32,
) {
    draw_label(ctx, MODE_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let options = [ParticleEmissionMode::Burst, ParticleEmissionMode::Continuous];
    let current = emitter.config.mode.to_string();
    if let Some(mode) = Dropdown::new(
        module.widget_ids.mode,
        field_rect(ctx, rect, *y),
        current.as_str(),
        &options,
        |value| value.to_string(),
    )
    .suppressed(blocked)
    .show(ctx)
    {
        update_config(module, emitter, |config| config.mode = mode);
        module.was_editing.set(true);
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

fn draw_visual_field(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    sprite_assets: &mut SpriteAssetContext,
    y: &mut f32,
) {
    draw_label(ctx, VISUAL_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let options = [VisualKind::Pixel, VisualKind::Line, VisualKind::Sprite];
    let current = VisualKind::from_visual(&emitter.config.visual).to_string();
    if let Some(visual) = Dropdown::new(
        module.widget_ids.visual,
        field_rect(ctx, rect, *y),
        &current,
        &options,
        |value| value.to_string(),
    )
    .suppressed(blocked)
    .show(ctx)
    {
        let visual = match visual {
            VisualKind::Pixel => Some(ParticleVisual::Pixel),
            VisualKind::Line => Some(ParticleVisual::Line),
            VisualKind::Sprite => pick_sprite_frame(ctx, sprite_assets)
                .map(|frame| ParticleVisual::Sprite { frame }),
        };
        if let Some(visual) = visual {
            update_config(module, emitter, |config| config.visual = visual);
            module.was_editing.set(true);
        }
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

fn draw_color_field(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    y: &mut f32,
) {
    draw_label(ctx, TINT_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let response = ColorInput::new(
        module.widget_ids.color,
        field_rect(ctx, rect, *y),
        emitter.config.color,
    )
    .blocked(blocked)
    .show(ctx);
    match response.edit {
        ColorInputEdit::Preview { .. } | ColorInputEdit::Commit { .. } => {
            update_config(module, emitter, |config| config.color = response.color);
            module.was_editing.set(true);
        }
        ColorInputEdit::Cancel { initial } => {
            update_config(module, emitter, |config| config.color = initial);
            module.was_editing.set(true);
        }
        ColorInputEdit::None if response.color != emitter.config.color => {
            update_config(module, emitter, |config| config.color = response.color);
            module.was_editing.set(true);
        }
        ColorInputEdit::None => {}
    }
    *y += ROW_HEIGHT + FIELD_GAP;

    draw_label(ctx, TINT_ALPHA_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let alpha = emitter.config.color.a;
    let (updated_alpha, commit) = NumberInput::new(
        module.widget_ids.color_alpha,
        field_rect(ctx, rect, *y),
        alpha,
    )
    .min(0.0)
    .max(1.0)
    .blocked(blocked)
    .show(ctx);
    record_commit(module, commit);
    if updated_alpha != alpha {
        update_config(module, emitter, |config| config.color.a = updated_alpha);
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

fn draw_sprite_frame_field(
    module: &mut ParticleEmitterModule,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    emitter: &mut ParticleEmitter,
    sprite_assets: &mut SpriteAssetContext,
    y: &mut f32,
) {
    draw_label(ctx, SPRITE_FRAME_LABEL, rect.x, *y, colors::DEFAULT_TEXT_COLOR);
    let field = field_rect(ctx, rect, *y);
    let preview_size = ROW_HEIGHT;
    let preview_gap = layout::WIDGET_SPACING;
    let button_rect = Rect::new(
        field.x,
        *y,
        field.w - preview_size - preview_gap,
        ROW_HEIGHT,
    );
    if Button::new(button_rect, "Pick sprite")
        .tooltip("Select a sprite image independently of the emitter entity.")
        .interaction_id(module.widget_ids.sprite_frame)
        .blocked(blocked)
        .show_native_dialog(ctx)
    {
        if let Some(frame) = pick_sprite_frame(ctx, sprite_assets) {
            update_config(module, emitter, |config| {
                config.visual = ParticleVisual::Sprite { frame };
            });
            module.was_editing.set(true);
        }
    }

    if let ParticleVisual::Sprite { frame } = &emitter.config.visual {
        let preview_rect = Rect::new(
            field.x + field.w - preview_size,
            *y,
            preview_size,
            preview_size,
        );
        let scale = (preview_size / frame.frame_size.x)
            .min(preview_size / frame.frame_size.y)
            .min(1.0);
        let destination_size = frame.frame_size * scale;
        let source = Rect::new(
            frame.col as f32 * frame.frame_size.x,
            frame.row as f32 * frame.frame_size.y,
            frame.frame_size.x,
            frame.frame_size.y,
        );
        let texture = sprite_assets
            .sprite_manager
            .get_texture_from_id(ctx, frame.sprite_id);
        ctx.draw_texture_ex(
            texture,
            preview_rect.x + (preview_size - destination_size.x) / 2.0,
            preview_rect.y + (preview_size - destination_size.y) / 2.0,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(destination_size),
                source: Some(source),
                flip_x: frame.flip_x,
                ..Default::default()
            },
        );
        ctx.draw_rectangle_lines(
            preview_rect.x,
            preview_rect.y,
            preview_rect.w,
            preview_rect.h,
            2.0,
            Color::WHITE,
        );
    }
    *y += ROW_HEIGHT + FIELD_GAP;
}

fn update_config(
    module: &mut ParticleEmitterModule,
    emitter: &mut ParticleEmitter,
    update: impl FnOnce(&mut ParticleEmitterConfig),
) {
    let mut candidate = emitter.config.clone();
    update(&mut candidate);
    match apply_config_candidate(emitter, candidate) {
        Ok(()) => module.validation_issues.borrow_mut().clear(),
        Err(issues) => *module.validation_issues.borrow_mut() = issues,
    }
}

fn pick_sprite_frame(
    ctx: &mut WgpuContext,
    sprite_assets: &mut SpriteAssetContext,
) -> Option<CurrentFrameSnapshot> {
    let path = rfd::FileDialog::new()
        .add_filter("PNG images", &["png"])
        .set_directory(assets_folder())
        .pick_file()?;
    let normalized_path = sprite_assets.sprite_manager.normalize_path(path);
    let sprite_id = sprite_assets.sprite_manager.get_or_load(
        sprite_assets.asset_registry,
        ctx,
        &normalized_path,
    )?;
    let (width, height) = sprite_assets.sprite_manager.texture_size(sprite_id)?;
    Some(CurrentFrameSnapshot {
        clip_id: ClipId::Idle,
        col: 0,
        row: 0,
        offset: Vec2::ZERO,
        sprite_id,
        frame_size: Vec2::new(width, height),
        flip_x: false,
    })
}

pub(super) fn draw_label(
    ctx: &mut WgpuContext,
    label: &str,
    x: f32,
    y: f32,
    color: Color,
) {
    let widget = Label::at(label, x, y + LABEL_Y_OFFSET)
        .font_size(layout::FIELD_TEXT_SIZE_16)
        .text_color(color);
    if let Some(tooltip) = tooltips::field_tooltip(label) {
        widget.tooltip(tooltip).show(ctx);
    } else {
        widget.show(ctx);
    }
}

pub(super) fn field_rect(ctx: &mut WgpuContext, rect: Rect, y: f32) -> Rect {
    let label_width = label_width(ctx);
    Rect::new(
        rect.x + label_width + layout::WIDGET_SPACING,
        y,
        rect.w - label_width - layout::WIDGET_SPACING,
        ROW_HEIGHT,
    )
}

pub(super) fn pair_rect(ctx: &mut WgpuContext, rect: Rect, y: f32, pair_index: usize) -> Rect {
    let fields = field_rect(ctx, rect, y);
    let gap = layout::WIDGET_SPACING;
    let width = (fields.w - gap) / 2.0;
    Rect::new(fields.x + pair_index as f32 * (width + gap), y, width, ROW_HEIGHT)
}

pub(super) fn record_commit(module: &mut ParticleEmitterModule, commit: InputCommit) {
    if matches!(commit, InputCommit::Previewing | InputCommit::Committed) {
        module.was_editing.set(true);
    }
}

pub(super) fn bounded_number_input<T>(
    id: WidgetId,
    rect: Rect,
    value: T,
    blocked: bool,
    min: Option<T>,
    max: Option<T>,
) -> NumberInput<T>
where
    T: Copy + PartialEq + PartialOrd + FromStr + Display + Default,
    <T as FromStr>::Err: Debug,
{
    let mut input = NumberInput::new(id, rect, value).blocked(blocked);
    if let Some(min) = min {
        input = input.min(min);
    }
    if let Some(max) = max {
        input = input.max(max);
    }
    input
}

fn label_width(ctx: &mut WgpuContext) -> f32 {
    [
        MODE_LABEL,
        VISUAL_LABEL,
        AREA_LABEL,
        LIFETIME_LABEL,
        VELOCITY_X_LABEL,
        VELOCITY_Y_LABEL,
        SCALE_LABEL,
        TINT_LABEL,
        TINT_ALPHA_LABEL,
        OPACITY_LABEL,
        LINE_LENGTH_LABEL,
        SPRITE_FRAME_LABEL,
        BURST_COUNT_LABEL,
        EMISSION_RATE_LABEL,
        PARTICLE_LIMIT_LABEL,
        SEED_LABEL,
    ]
    .into_iter()
    .map(|label| measure_text(ctx, label, layout::FIELD_TEXT_SIZE_16).width)
    .fold(0.0, f32::max)
}

impl VisualKind {
    fn from_visual(visual: &ParticleVisual) -> Self {
        match visual {
            ParticleVisual::Pixel => Self::Pixel,
            ParticleVisual::Line => Self::Line,
            ParticleVisual::Sprite { .. } => Self::Sprite,
        }
    }
}
