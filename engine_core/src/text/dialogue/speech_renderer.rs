use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::Pivot;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{SpeechBubble, SubPixel, Transform};
use crate::rendering::helpers::entity_dimensions;
use crate::rendering::helpers::lerp_position;
use crate::rendering::helpers::visual_position;
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
    sprite_manager: &SpriteManager,
    current_room: RoomId,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> Vec<SpeechBubbleRenderData> {
    let mut bubbles = Vec::new();
    let bubble_store = ecs.get_store::<SpeechBubble>();
    let transform_store = ecs.get_store::<Transform>();
    let sub_pixel_store = ecs.get_store::<SubPixel>();

    for &entity in ecs.entities_in_room(current_room) {
        ecs.assert_room_membership(current_room, entity);

        let Some(bubble) = bubble_store.get(entity) else {
            continue;
        };
        let Some(transform) = transform_store.get(entity) else {
            continue;
        };

        let current_pos = visual_position(transform.position, sub_pixel_store.get(entity));
        let world_pos = interpolate_position(
            entity,
            current_pos,
            alpha,
            prev_positions,
        );
        let entity_size = entity_dimensions(ecs, sprite_manager, entity, grid_size);

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
/// Call this after drawing the world with the render camera used for the room.
pub fn render_speech_bubbles<C: BishopContext>(
    ctx: &mut C,
    bubbles: &[SpeechBubbleRenderData],
    config: &DialogueConfig,
    render_cam: &Camera2D,
) {
    let projection = screen_space_projection(render_cam, ctx.screen_width(), ctx.screen_height());

    for bubble in bubbles {
        render_bubble_screen_space(ctx, bubble, config, &projection);
    }
}

/// Shared screen-space projection derived from the current render camera and viewport.
struct ScreenSpaceProjection<'a> {
    render_cam: &'a Camera2D,
    screen_size: Vec2,
    screen_scale: Vec2,
}

fn screen_space_projection(
    render_cam: &Camera2D,
    screen_w: f32,
    screen_h: f32,
) -> ScreenSpaceProjection<'_> {
    let screen_size = Vec2::new(screen_w, screen_h);
    let viewport_size = render_cam
        .viewport
        .map(|(_, _, width, height)| Vec2::new(width as f32, height as f32))
        .unwrap_or(screen_size);
    let half_w = 1.0 / render_cam.zoom.x;
    let half_h = 1.0 / render_cam.zoom.y;

    ScreenSpaceProjection {
        render_cam,
        screen_size,
        screen_scale: Vec2::new(viewport_size.x / (2.0 * half_w), viewport_size.y / (2.0 * half_h)),
    }
}

/// Renders a single speech bubble in screen space.
fn render_bubble_screen_space<C: BishopContext>(
    ctx: &mut C,
    bubble: &SpeechBubbleRenderData,
    config: &DialogueConfig,
    projection: &ScreenSpaceProjection<'_>,
) {
    let text_scale = projection.screen_scale.x.min(projection.screen_scale.y);
    let font_size = bubble.font_size.unwrap_or(config.font_size) * text_scale;
    let max_width = bubble.max_width.unwrap_or(config.max_width) * text_scale;
    let padding = config.padding * text_scale;

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
    let entity_width_scaled = bubble.entity_size.x * projection.screen_scale.x;
    let entity_height_scaled = bubble.entity_size.y * projection.screen_scale.y;

    let screen_pos = projection.render_cam.world_to_screen(
        bubble.world_pos,
        projection.screen_size.x,
        projection.screen_size.y,
    );

    let entity_top_center_x =
        screen_pos.x - entity_width_scaled * pivot_offset.x + entity_width_scaled / 2.0;
    let entity_top_y = screen_pos.y - entity_height_scaled * pivot_offset.y;

    let bubble_x =
        entity_top_center_x - bubble_width / 2.0 + bubble.offset.0 * projection.screen_scale.x;
    let bubble_y = entity_top_y + bubble.offset.1 * projection.screen_scale.y - bubble_height;

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
        return lerp_position(*prev_pos, current_pos, alpha);
    }
    current_pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_bubble_projection_uses_camera_viewport_dimensions() {
        let camera = Camera2D {
            zoom: Vec2::new(2.0 / 320.0, 2.0 / 180.0),
            viewport: Some((90, 0, 1920, 1080)),
            ..Default::default()
        };

        let projection = screen_space_projection(&camera, 2100.0, 1080.0);

        assert_eq!(projection.screen_size, Vec2::new(2100.0, 1080.0));
        assert_eq!(projection.screen_scale, Vec2::new(6.0, 6.0));
    }

    #[test]
    fn collect_speech_bubbles_only_returns_entities_in_current_room() {
        let room_a = RoomId(1);
        let room_b = RoomId(2);
        let mut ecs = Ecs::default();

        let in_room = ecs.create_entity()
            .with(Transform::default())
            .with(SpeechBubble::default())
            .with_current_room(room_a)
            .finish();

        ecs.create_entity()
            .with(Transform::default())
            .with(SpeechBubble::default())
            .with_current_room(room_b)
            .finish();

        let bubbles = collect_speech_bubbles(
            &ecs,
            &SpriteManager::default(),
            room_a,
            1.0,
            None,
            16.0,
        );

        assert_eq!(bubbles.len(), 1);
        assert_eq!(ecs.get::<crate::ecs::CurrentRoom>(in_room).map(|room| room.0), Some(room_a));
    }
}
