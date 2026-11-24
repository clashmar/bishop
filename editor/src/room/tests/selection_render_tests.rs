use super::super::*;
use crate::room::selection::selection_render_rect;

#[test]
fn odd_width_bottom_center_sprite_world_rect_matches_static_sprite_draw() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn even_width_bottom_center_sprite_world_rect_keeps_existing_alignment() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(24.0, 24.0)),
        None,
    );

    assert_eq!(top_left, vec2(-12.0, -24.0));
    assert_eq!(size, vec2(24.0, 24.0));
}

#[test]
fn static_sprite_branch_matches_sprite_size() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn current_frame_selection_uses_offset_without_snapping() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        Some(&CurrentFrame {
            sprite_id: SpriteId(2),
            frame_size: vec2(5.0, 5.0),
            offset: vec2(0.25, -0.75),
            ..Default::default()
        }),
    );

    assert_eq!(top_left, vec2(-2.25, -5.75));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn stale_current_frame_uses_static_sprite_fallback_size_and_position() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        Some(&CurrentFrame {
            sprite_id: SpriteId(0),
            frame_size: vec2(9.0, 9.0),
            offset: vec2(4.5, 4.5),
            ..Default::default()
        }),
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn unset_sprite_keeps_unsnapped_fallback_alignment() {
    let (top_left, size) =
        selection_render_rect(Vec2::ZERO, 8.0, Pivot::BottomCenter, false, None, None);

    assert_eq!(top_left, vec2(-4.0, -8.0));
    assert_eq!(size, vec2(8.0, 8.0));
}

#[test]
fn placeholder_selection_keeps_grid_centering_behavior() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        true,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-4.0, -4.0));
    assert_eq!(size, vec2(8.0, 8.0));
}
