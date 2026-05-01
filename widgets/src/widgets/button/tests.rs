use super::*;
use crate::constants::colors;
use crate::widgets::test_support::WidgetTestContext;

#[test]
fn primary_click_requires_matching_press_and_release() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 20.0);
    ctx.left_pressed = true;
    ctx.left_down = true;

    assert!(!Button::new(button, "Play").show(&mut ctx));

    reset_click_consumed();
    ctx.left_pressed = false;
    ctx.left_down = false;
    ctx.left_released = true;

    assert!(Button::new(button, "Play").show(&mut ctx));
}

#[test]
fn primary_click_does_not_activate_from_another_controls_press() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (120.0, 20.0);
    ctx.left_pressed = true;
    ctx.left_down = true;

    assert!(!Button::new(button, "Play").show(&mut ctx));

    reset_click_consumed();
    ctx.left_pressed = false;
    ctx.left_down = false;
    ctx.left_released = true;
    ctx.mouse_pos = (40.0, 20.0);

    assert!(!Button::new(button, "Play").show(&mut ctx));
}

#[test]
fn secondary_clicks_are_reported_when_opted_in() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 20.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let clicks = Button::new(button, "Play")
        .allow_secondary_click()
        .show_clicks(&mut ctx, WidgetVisuals::default());

    assert!(!clicks.primary);
    assert!(!clicks.secondary);

    reset_click_consumed();
    ctx.right_pressed = false;
    ctx.right_down = false;
    ctx.right_released = true;

    let clicks = Button::new(button, "Play")
        .allow_secondary_click()
        .show_clicks(&mut ctx, WidgetVisuals::default());

    assert!(!clicks.primary);
    assert!(clicks.secondary);
}

#[test]
fn blocked_buttons_are_dimmed_and_do_not_click() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 20.0);
    ctx.left_pressed = true;
    ctx.left_down = true;

    assert!(!Button::new(button, "Play").blocked(true).show(&mut ctx));
    assert_eq!(
        ctx.rectangle_fills.last().copied(),
        Some(BLOCKED_BACKGROUND_COLOR)
    );
    assert_eq!(
        ctx.rectangle_lines.last().copied(),
        Some(BLOCKED_OUTLINE_COLOR)
    );
    assert_eq!(ctx.text_colors.last().copied(), Some(BLOCKED_TEXT_COLOR));
}

#[test]
fn suppressed_buttons_are_not_dimmed_and_do_not_click() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 20.0);
    ctx.left_pressed = true;
    ctx.left_down = true;

    assert!(!Button::new(button, "Play").suppressed(true).show(&mut ctx));
    assert_eq!(
        ctx.rectangle_fills.last().copied(),
        Some(colors::DEFAULT_SURFACE_COLOR)
    );
    assert_eq!(
        ctx.rectangle_lines.last().copied(),
        Some(colors::DEFAULT_BORDER_COLOR)
    );
    assert_eq!(
        ctx.text_colors.last().copied(),
        Some(colors::DEFAULT_TEXT_COLOR)
    );
}

#[test]
fn blocked_and_suppressed_buttons_keep_blocked_visuals() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 20.0);

    assert!(!Button::new(button, "Play")
        .blocked(true)
        .suppressed(true)
        .show(&mut ctx));
    assert_eq!(
        ctx.rectangle_fills.last().copied(),
        Some(BLOCKED_BACKGROUND_COLOR)
    );
    assert_eq!(
        ctx.rectangle_lines.last().copied(),
        Some(BLOCKED_OUTLINE_COLOR)
    );
    assert_eq!(ctx.text_colors.last().copied(), Some(BLOCKED_TEXT_COLOR));
}

#[test]
fn deferred_button_activation_waits_until_next_idle_frame() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let interaction_id = WidgetId(4242);

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (40.0, 20.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    widgets_frame_start(&mut press_ctx);
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut press_ctx));
    widgets_frame_end(&mut press_ctx);

    reset_click_consumed();
    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (40.0, 20.0);
    release_ctx.left_released = true;

    widgets_frame_start(&mut release_ctx);
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut release_ctx));
    widgets_frame_end(&mut release_ctx);

    reset_click_consumed();
    let mut idle_ctx = WidgetTestContext::new();
    idle_ctx.mouse_pos = (40.0, 20.0);

    widgets_frame_start(&mut idle_ctx);
    assert!(Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut idle_ctx));
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut idle_ctx));
    widgets_frame_end(&mut idle_ctx);
}

#[test]
fn deferred_button_activation_expires_after_first_idle_frame_when_unclaimed() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let interaction_id = WidgetId(5151);

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (40.0, 20.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    widgets_frame_start(&mut press_ctx);
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut press_ctx));
    widgets_frame_end(&mut press_ctx);

    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (40.0, 20.0);
    release_ctx.left_released = true;

    widgets_frame_start(&mut release_ctx);
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut release_ctx));
    widgets_frame_end(&mut release_ctx);

    let mut other_idle_ctx = WidgetTestContext::new();
    widgets_frame_start(&mut other_idle_ctx);
    assert!(!Button::new(button, "Other").show_native_dialog(&mut other_idle_ctx));
    widgets_frame_end(&mut other_idle_ctx);

    let mut later_idle_ctx = WidgetTestContext::new();
    widgets_frame_start(&mut later_idle_ctx);
    assert!(!Button::new(button, "Pick")
        .interaction_id(interaction_id)
        .show_native_dialog(&mut later_idle_ctx));
    widgets_frame_end(&mut later_idle_ctx);
}

#[test]
fn deferred_button_activation_does_not_leak_between_targets() {
    reset_click_consumed();

    let button = Rect::new(0.0, 0.0, 80.0, 30.0);
    let first = WidgetId(6001);
    let second = WidgetId(6002);

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (40.0, 20.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    widgets_frame_start(&mut press_ctx);
    assert!(!Button::new(button, "Pick A")
        .interaction_id(first)
        .show_native_dialog(&mut press_ctx));
    widgets_frame_end(&mut press_ctx);

    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (40.0, 20.0);
    release_ctx.left_released = true;

    widgets_frame_start(&mut release_ctx);
    assert!(!Button::new(button, "Pick A")
        .interaction_id(first)
        .show_native_dialog(&mut release_ctx));
    widgets_frame_end(&mut release_ctx);

    let mut idle_ctx = WidgetTestContext::new();
    widgets_frame_start(&mut idle_ctx);
    assert!(!Button::new(button, "Pick B")
        .interaction_id(second)
        .show_native_dialog(&mut idle_ctx));
    assert!(Button::new(button, "Pick A")
        .interaction_id(first)
        .show_native_dialog(&mut idle_ctx));
    widgets_frame_end(&mut idle_ctx);
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::{Theme, WidgetThemeMapper};

    #[test]
    fn button_theme_mapper_maps_key_roles() {
        let theme = Theme {
            background: Color::GREEN,
            text: Color::BLUE,
            text_muted: Color::new(0.5, 0.5, 0.5, 1.0),
            border: Color::new(0.8, 0.8, 0.8, 1.0),
            hover: Color::new(0.2, 0.2, 1.0, 1.0),
            ..Theme::default()
        };
        let visuals = Button::theme_visuals(&theme);
        assert_eq!(visuals.background, Some(Color::GREEN));
        assert_eq!(visuals.text, Some(Color::BLUE));
        assert_eq!(visuals.text_muted, Some(Color::new(0.5, 0.5, 0.5, 1.0)));
        assert_eq!(visuals.border, Some(Color::new(0.8, 0.8, 0.8, 1.0)));
        assert_eq!(visuals.hover, Some(Color::new(0.2, 0.2, 1.0, 1.0)));
        assert_eq!(visuals.primary, None);
        assert_eq!(visuals.danger, None);
        assert_eq!(visuals.surface, None);
        assert_eq!(visuals.accent, None);
    }
}
