// engine_core/src/text/dialogue/speech_renderer.rs
use crate::assets::asset_manager::AssetManager;
use crate::camera::game_camera::*;
use crate::ecs::Pivot;
use crate::ecs::component::CurrentRoom;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::transform::Transform;
use crate::rendering::helpers::lerp_rounded;
use crate::rendering::helpers::entity_dimensions;
use crate::text::*;
use crate::ui::text::*;
use crate::worlds::room::RoomId;
use bishop::prelude::*;
use std::collections::HashMap;

/// Collected data for rendering a speech bubble in screen space.
pub struct SpeechBubbleRenderData {
    pub text: String,
    pub world_pos: Vec2,
    pub entity_size: Vec2,
    pub pivot: Pivot,
    pub color: [f32; 4],
    pub offset: (f32, f32),
    pub font_size: Option<f32>,
    pub max_width: Option<f32>,
    pub show_background: bool,
    pub background_color: [f32; 4],
}

/// Collects speech bubble data for entities in the current room.
/// Returns data needed for screen-space rendering.
pub fn collect_speech_bubbles(
    ecs: &Ecs,
    asset_manager: &AssetManager,
    current_room: RoomId,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> Vec<SpeechBubbleRenderData> {
    let mut bubbles = Vec::new();
    let bubble_store = ecs.get_store::<SpeechBubble>();
    let transform_store = ecs.get_store::<Transform>();
    let room_store = ecs.get_store::<CurrentRoom>();

    for (entity, bubble) in &bubble_store.data {
        if let Some(current) = room_store.get(*entity) {
            if current.0 != current_room {
                continue;
            }
        } else {
            continue;
        }

        let Some(transform) = transform_store.get(*entity) else {
            continue;
        };

        let world_pos = interpolate_position(*entity, transform.position, alpha, prev_positions);
        let entity_size = entity_dimensions(ecs, asset_manager, *entity, grid_size);

        bubbles.push(SpeechBubbleRenderData {
            text: bubble.text.clone(),
            world_pos,
            entity_size,
            pivot: transform.pivot,
            color: bubble.color,
            offset: bubble.offset,
            font_size: bubble.font_size,
            max_width: bubble.max_width,
            show_background: bubble.show_background,
            background_color: bubble.background_color,
        });
    }

    bubbles
}

/// Renders speech bubbles in screen space for crisp text.
/// Call this AFTER present_game() with the render camera used for the room.
pub fn render_speech_bubbles<C: BishopContext>(
    ctx: &mut C,
    bubbles: &[SpeechBubbleRenderData],
    config: &DialogueConfig,
    render_cam: &Camera2D,
    grid_size: f32,
) {
    let virt_w = world_virtual_width(grid_size);
    let virt_h = world_virtual_height(grid_size);
    let win_w = ctx.screen_width();
    let win_h = ctx.screen_height();

    let scale_w = win_w / virt_w;
    let scaled_h = virt_h * scale_w;

    let (scale, offset) = if scaled_h <= win_h {
        (scale_w, Vec2::new(0.0, (win_h - scaled_h) / 2.0))
    } else {
        let scale_h = win_h / virt_h;
        let scaled_w = virt_w * scale_h;
        (scale_h, Vec2::new((win_w - scaled_w) / 2.0, 0.0))
    };
    let projection = ScreenSpaceProjection {
        render_cam,
        virt_size: Vec2::new(virt_w, virt_h),
        scale,
        offset,
    };

    for bubble in bubbles {
        render_bubble_screen_space(ctx, bubble, config, &projection);
    }
}

/// Shared screen-space projection derived from the current render camera and viewport.
struct ScreenSpaceProjection<'a> {
    render_cam: &'a Camera2D,
    virt_size: Vec2,
    scale: f32,
    offset: Vec2,
}

/// Renders a single speech bubble in screen space.
fn render_bubble_screen_space<C: BishopContext>(
    ctx: &mut C,
    bubble: &SpeechBubbleRenderData,
    config: &DialogueConfig,
    projection: &ScreenSpaceProjection<'_>,
) {
    let font_size = bubble.font_size.unwrap_or(config.font_size) * projection.scale;
    let max_width = bubble.max_width.unwrap_or(config.max_width) * projection.scale;
    let padding = config.padding * projection.scale;

    let lines = wrap_text(ctx, &bubble.text, max_width, font_size);
    if lines.is_empty() {
        return;
    }

    let line_height = font_size * 1.2;
    let total_text_height = lines.len() as f32 * line_height;

    let max_line_width = lines
        .iter()
        .map(|line| measure_text(ctx, line, font_size).width)
        .fold(0.0_f32, f32::max);

    let bubble_width = max_line_width + padding * 2.0;
    let bubble_height = total_text_height + padding * 2.0;

    let pivot_offset = bubble.pivot.as_normalized();
    let entity_width_scaled = bubble.entity_size.x * projection.scale;
    let entity_height_scaled = bubble.entity_size.y * projection.scale;

    let half_w = 1.0 / projection.render_cam.zoom.x;
    let half_h = 1.0 / projection.render_cam.zoom.y;
    let virt_x = (bubble.world_pos.x - projection.render_cam.target.x + half_w)
        / (2.0 * half_w)
        * projection.virt_size.x;
    let virt_y = (bubble.world_pos.y - projection.render_cam.target.y + half_h)
        / (2.0 * half_h)
        * projection.virt_size.y;

    let screen_x = virt_x * projection.scale + projection.offset.x;
    let screen_y = virt_y * projection.scale + projection.offset.y;

    let entity_top_center_x =
        screen_x - entity_width_scaled * pivot_offset.x + entity_width_scaled / 2.0;
    let entity_top_y = screen_y - entity_height_scaled * pivot_offset.y;

    let bubble_x =
        entity_top_center_x - bubble_width / 2.0 + bubble.offset.0 * projection.scale;
    let bubble_y = entity_top_y + bubble.offset.1 * projection.scale - bubble_height;

    if bubble.show_background {
        let bg_color = Color::new(
            bubble.background_color[0],
            bubble.background_color[1],
            bubble.background_color[2],
            bubble.background_color[3],
        );
        ctx.draw_rectangle(bubble_x, bubble_y, bubble_width, bubble_height, bg_color);
    }

    let text_color = Color::new(
        bubble.color[0],
        bubble.color[1],
        bubble.color[2],
        bubble.color[3],
    );

    for (i, line) in lines.iter().enumerate() {
        let line_width = measure_text(ctx, line, font_size).width;
        let text_x = bubble_x + (bubble_width - line_width) / 2.0;
        let text_y = bubble_y + padding + (i as f32 + 1.0) * line_height - line_height * 0.2;

        ctx.draw_text(line, text_x, text_y, font_size, text_color);
    }
}

/// Wraps text to fit within a maximum width.
fn wrap_text<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    max_width: f32,
    font_size: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        let test_width = measure_text(ctx, &test_line, font_size).width;

        if test_width <= max_width || current_line.is_empty() {
            current_line = test_line;
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() && !text.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

/// Interpolates position for smooth rendering.
fn interpolate_position(
    entity: Entity,
    current_pos: Vec2,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) -> Vec2 {
    if let Some(prev_map) = prev_positions
        && let Some(prev_pos) = prev_map.get(&entity)
    {
        return lerp_rounded(*prev_pos, current_pos, alpha);
    }
    current_pos
}
