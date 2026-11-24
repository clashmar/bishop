use super::{
    ColorInputEdit, PICKER_STATE, color_picker_open, hsv_to_rgb, rgb_to_hsv,
};
use super::drawing::picker_layout;
use crate::test_utils::WidgetTestContext;
use crate::{
    Checkbox, Color, ColorInput, KeyCode, Rect, Widget, WidgetId, color_input_reset,
    reset_click_consumed, widgets_frame_end, widgets_frame_start,
};

const INPUT_RECT: Rect = Rect::new(0.0, 0.0, 200.0, 30.0);
const SWATCH_RECT: Rect = Rect::new(0.0, 0.0, 30.0, 30.0);

fn open_picker(id: WidgetId, color: Color, opacity_enabled: bool) {
    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (15.0, 15.0);
    context.left_pressed = true;
    context.left_down = true;
    ColorInput::new(id, INPUT_RECT, color)
        .opacity_enabled(opacity_enabled)
        .show(&mut context);
}

#[test]
fn clicking_color_swatch_opens_picker_with_initial_color() {
    let id = WidgetId(0xC010_3001);
    let mut initial = Color::from_hex("4080BF");
    initial.a = 0.4;
    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (15.0, 15.0);
    context.left_pressed = true;
    context.left_down = true;

    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert_eq!(response.color, initial);
    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn picker_is_drawn_after_later_editor_widgets() {
    const MARKER: Rect = Rect::new(700.0, 500.0, 1.0, 1.0);
    let id = WidgetId(0xC010_3012);
    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (15.0, 15.0);
    context.left_pressed = true;
    context.left_down = true;
    ColorInput::new(id, INPUT_RECT, Color::WHITE).show(&mut context);
    let expected_popup = picker_layout(&context, SWATCH_RECT, true).popup;

    context.rect_calls.push(MARKER);
    widgets_frame_end(&mut context);

    let marker_index = context.rect_calls.iter().position(|rect| *rect == MARKER);
    let popup_index = context.rect_calls.iter().position(|rect| *rect == expected_popup);
    assert!(popup_index > marker_index);
    color_input_reset(id);
}

#[test]
fn pressing_escape_cancels_open_picker() {
    let id = WidgetId(0xC010_3002);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);

    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.keys_pressed.push(KeyCode::Escape);
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert_eq!(response.color, initial);
    assert_eq!(response.edit, ColorInputEdit::Cancel { initial });
    color_input_reset(id);
}

#[test]
fn clicking_outside_commits_open_picker() {
    let id = WidgetId(0xC010_3003);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);

    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (400.0, 400.0);
    context.left_pressed = true;
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert_eq!(response.color, initial);
    assert_eq!(response.edit, ColorInputEdit::Commit { initial });
    color_input_reset(id);
}

#[test]
fn outside_click_dismisses_picker_before_its_owner_is_drawn() {
    let id = WidgetId(0xC010_3015);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, true);
    let Some(opacity) = layout.opacity else {
        panic!("opacity control should be enabled");
    };

    let mut preview_context = WidgetTestContext::new();
    preview_context.mouse_pos = (
        opacity.x + opacity.w * 0.7,
        opacity.y + opacity.h / 2.0,
    );
    preview_context.left_pressed = true;
    preview_context.left_down = true;
    widgets_frame_start(&mut preview_context);
    let preview = ColorInput::new(id, INPUT_RECT, initial).show(&mut preview_context);
    assert!((preview.color.a - 0.7).abs() < 0.02);

    let mut context = WidgetTestContext::new();
    context.mouse_pos = (400.0, 400.0);
    context.left_pressed = true;
    widgets_frame_start(&mut context);

    assert!(!PICKER_STATE.with(|state| state.borrow().contains_key(&id)));
    assert!(color_picker_open());
    assert!(crate::color_picker_blocks_canvas());
    assert!(crate::is_click_consumed());

    let mut next_frame = WidgetTestContext::new();
    widgets_frame_start(&mut next_frame);
    assert!(!crate::color_picker_blocks_canvas());
    let response = ColorInput::new(id, INPUT_RECT, preview.color).show(&mut next_frame);
    assert_eq!(response.color, preview.color);
    assert_eq!(response.edit, ColorInputEdit::Commit { initial });
    color_input_reset(id);
}

#[test]
fn consumed_outside_click_still_dismisses_open_picker() {
    let id = WidgetId(0xC010_3016);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);

    reset_click_consumed();
    crate::consume_click();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (400.0, 400.0);
    context.left_pressed = true;
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert_eq!(response.edit, ColorInputEdit::Commit { initial });
    color_input_reset(id);
}

#[test]
fn frame_start_click_in_inline_hex_field_keeps_picker_open() {
    let id = WidgetId(0xC010_3017);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);

    let mut context = WidgetTestContext::new();
    context.mouse_pos = (80.0, 15.0);
    context.left_pressed = true;
    widgets_frame_start(&mut context);

    assert!(color_picker_open());
    assert!(crate::color_picker_blocks_canvas());
    color_input_reset(id);
}

#[test]
fn dragging_saturation_value_updates_color_preview() {
    let id = WidgetId(0xC010_3004);
    let initial = Color::new(0.2, 0.2, 0.2, 0.5);
    open_picker(id, initial, true);
    reset_click_consumed();
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, true);

    let mut context = WidgetTestContext::new();
    context.mouse_pos = (
        layout.saturation_value.x + layout.saturation_value.w - 1.0,
        layout.saturation_value.y + 1.0,
    );
    context.left_pressed = true;
    context.left_down = true;
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert!(response.color.r > 0.95);
    assert!(response.color.g < 0.05);
    assert!(response.color.b < 0.05);
    assert_eq!(response.color.a, initial.a);
    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn dragging_opacity_updates_alpha_live() {
    let id = WidgetId(0xC010_3005);
    let initial = Color::from_hex("CC3366");
    open_picker(id, initial, true);
    reset_click_consumed();
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, true);
    let Some(opacity) = layout.opacity else {
        panic!("opacity control should be enabled");
    };

    let mut context = WidgetTestContext::new();
    context.mouse_pos = (opacity.x + opacity.w - 1.0, opacity.y + opacity.h / 2.0);
    context.left_pressed = true;
    context.left_down = true;
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert!(response.color.a > 0.95);
    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn dragging_picker_control_across_frames_keeps_the_live_preview() {
    let id = WidgetId(0xC010_3014);
    let initial = Color::from_hex("CC3366");
    open_picker(id, initial, true);
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, true);
    let Some(opacity) = layout.opacity else {
        panic!("opacity control should be enabled");
    };

    reset_click_consumed();
    let mut first_frame = WidgetTestContext::new();
    first_frame.mouse_pos = (opacity.x + opacity.w * 0.3, opacity.y + opacity.h / 2.0);
    first_frame.left_pressed = true;
    first_frame.left_down = true;
    let first = ColorInput::new(id, INPUT_RECT, initial).show(&mut first_frame);

    reset_click_consumed();
    let mut second_frame = WidgetTestContext::new();
    second_frame.mouse_pos = (opacity.x + opacity.w * 0.65, opacity.y + opacity.h / 2.0);
    second_frame.left_down = true;
    let second = ColorInput::new(id, INPUT_RECT, first.color).show(&mut second_frame);

    assert!((first.color.a - 0.3).abs() < 0.02);
    assert!((second.color.a - 0.65).abs() < 0.02);
    assert_eq!(second.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn clicking_hex_field_keeps_picker_open_for_direct_editing() {
    let id = WidgetId(0xC010_3008);
    let initial = Color::new(0.2, 0.4, 0.6, 0.35);
    open_picker(id, initial, true);

    reset_click_consumed();
    let mut press = WidgetTestContext::new();
    press.mouse_pos = (80.0, 15.0);
    press.left_pressed = true;
    press.left_down = true;
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut press);

    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    assert!(PICKER_STATE.with(|state| state.borrow().contains_key(&id)));

    reset_click_consumed();
    let mut release = WidgetTestContext::new();
    release.mouse_pos = (80.0, 15.0);
    release.left_released = true;
    ColorInput::new(id, INPUT_RECT, initial).show(&mut release);

    let mut select_all = WidgetTestContext::new();
    select_all.keys_down.push(KeyCode::LeftControl);
    select_all.keys_pressed.push(KeyCode::A);
    ColorInput::new(id, INPUT_RECT, initial).show(&mut select_all);

    let mut type_hex = WidgetTestContext::new();
    type_hex.chars = "FF0000".chars().collect();
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut type_hex);

    assert_eq!(response.color, Color::new(1.0, 0.0, 0.0, initial.a));
    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn escape_cancels_hex_preview_and_restores_the_inline_text() {
    let id = WidgetId(0xC010_3013);
    let initial = Color::new(0.2, 0.4, 0.6, 0.35);
    open_picker(id, initial, true);

    reset_click_consumed();
    let mut press = WidgetTestContext::new();
    press.mouse_pos = (80.0, 15.0);
    press.left_pressed = true;
    press.left_down = true;
    ColorInput::new(id, INPUT_RECT, initial).show(&mut press);

    reset_click_consumed();
    let mut release = WidgetTestContext::new();
    release.mouse_pos = (80.0, 15.0);
    release.left_released = true;
    ColorInput::new(id, INPUT_RECT, initial).show(&mut release);

    let mut select_all = WidgetTestContext::new();
    select_all.keys_down.push(KeyCode::LeftControl);
    select_all.keys_pressed.push(KeyCode::A);
    ColorInput::new(id, INPUT_RECT, initial).show(&mut select_all);

    let mut type_hex = WidgetTestContext::new();
    type_hex.chars = "FF0000".chars().collect();
    let preview = ColorInput::new(id, INPUT_RECT, initial).show(&mut type_hex);
    assert_eq!(preview.color, Color::new(1.0, 0.0, 0.0, initial.a));

    let mut escape = WidgetTestContext::new();
    escape.keys_pressed.push(KeyCode::Escape);
    let canceled = ColorInput::new(id, INPUT_RECT, preview.color).show(&mut escape);

    assert_eq!(canceled.color, initial);
    assert_eq!(canceled.edit, ColorInputEdit::Cancel { initial });
    let restored = ColorInput::new(id, INPUT_RECT, initial).show(&mut WidgetTestContext::new());
    assert_eq!(restored.color, initial);
    color_input_reset(id);
}

#[test]
fn blocked_color_input_does_not_open_picker() {
    let id = WidgetId(0xC010_3006);
    let initial = Color::from_hex("4080BF");
    reset_click_consumed();
    let mut context = WidgetTestContext::new();
    context.mouse_pos = (15.0, 15.0);
    context.left_pressed = true;
    context.left_down = true;

    let response = ColorInput::new(id, INPUT_RECT, initial)
        .blocked(true)
        .show(&mut context);

    assert_eq!(response.edit, ColorInputEdit::None);
    assert!(!PICKER_STATE.with(|state| state.borrow().contains_key(&id)));
    color_input_reset(id);
}

#[test]
fn popup_click_does_not_toggle_controls_behind_it() {
    let id = WidgetId(0xC010_3009);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, true);
    reset_click_consumed();
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, true);
    let pointer = (layout.saturation_value.x + 20.0, layout.saturation_value.y + 20.0);
    let rect = Rect::new(pointer.0 - 5.0, pointer.1 - 5.0, 10.0, 10.0);
    let mut value = false;
    let mut context = WidgetTestContext::new();
    context.mouse_pos = pointer;
    context.left_pressed = true;
    context.left_down = true;

    Checkbox::new(rect, &mut value).show(&mut context);
    let response = ColorInput::new(id, INPUT_RECT, initial).show(&mut context);

    assert!(!value);
    assert_eq!(response.edit, ColorInputEdit::Preview { initial });
    color_input_reset(id);
}

#[test]
fn resetting_color_input_closes_picker_state() {
    let id = WidgetId(0xC010_3010);
    open_picker(id, Color::from_hex("4080BF"), true);

    color_input_reset(id);

    assert!(!color_picker_open());
}

#[test]
fn disabling_opacity_hides_picker_opacity_control() {
    let id = WidgetId(0xC010_3007);
    let initial = Color::from_hex("4080BF");
    open_picker(id, initial, false);

    let opacity_hidden = PICKER_STATE.with(|state| {
        state
            .borrow()
            .get(&id)
            .is_some_and(|picker| picker.layout.opacity.is_none())
    });
    assert!(opacity_hidden);
    color_input_reset(id);
}

#[test]
fn dragging_hue_bar_changes_the_selected_color() {
    let id = WidgetId(0xC010_3011);
    let initial = Color::new(0.2, 0.2, 0.2, 1.0);
    open_picker(id, initial, false);
    reset_click_consumed();
    let layout = picker_layout(&WidgetTestContext::new(), SWATCH_RECT, false);

    let mut select_saturation = WidgetTestContext::new();
    select_saturation.mouse_pos = (
        layout.saturation_value.x + layout.saturation_value.w - 1.0,
        layout.saturation_value.y + 1.0,
    );
    select_saturation.left_pressed = true;
    select_saturation.left_down = true;
    ColorInput::new(id, INPUT_RECT, initial)
        .opacity_enabled(false)
        .show(&mut select_saturation);

    reset_click_consumed();
    let mut select_hue = WidgetTestContext::new();
    select_hue.mouse_pos = (
        layout.hue.x + layout.hue.w / 2.0,
        layout.hue.y + layout.hue.h / 3.0,
    );
    select_hue.left_pressed = true;
    select_hue.left_down = true;
    let response = ColorInput::new(id, INPUT_RECT, Color::new(1.0, 0.0, 0.0, 1.0))
        .opacity_enabled(false)
        .show(&mut select_hue);

    assert!(response.color.r < 0.05);
    assert!(response.color.g > 0.95);
    assert!(response.color.b < 0.05);
    color_input_reset(id);
}

#[test]
fn rgb_hsv_round_trip_preserves_primary_colors_and_grayscale() {
    for color in [
        Color::new(1.0, 0.0, 0.0, 0.25),
        Color::new(0.0, 1.0, 0.0, 0.5),
        Color::new(0.0, 0.0, 1.0, 0.75),
        Color::new(0.4, 0.4, 0.4, 0.6),
    ] {
        let (hue, saturation, value) = rgb_to_hsv(color);
        let round_trip = hsv_to_rgb(hue, saturation, value, color.a);

        assert!((round_trip.r - color.r).abs() <= 0.0001);
        assert!((round_trip.g - color.g).abs() <= 0.0001);
        assert!((round_trip.b - color.b).abs() <= 0.0001);
        assert_eq!(round_trip.a, color.a);
    }
}

#[test]
fn hsv_conversion_wraps_hue_and_clamps_channels() {
    let red = hsv_to_rgb(0.0, 1.0, 1.0, 0.4);
    let wrapped_red = hsv_to_rgb(360.0, 1.0, 1.0, 0.4);
    let clamped = hsv_to_rgb(0.0, 2.0, -1.0, 2.0);

    assert_eq!(red, wrapped_red);
    assert_eq!(clamped, Color::new(0.0, 0.0, 0.0, 1.0));
}

#[test]
fn picker_layout_at_window_edges_stays_inside_screen() {
    let mut context = WidgetTestContext::new();
    context.screen_width = 300.0;
    context.screen_height = 300.0;
    let layout = picker_layout(
        &context,
        Rect::new(290.0, 285.0, 10.0, 10.0),
        true,
    );

    assert!(layout.popup.x >= 0.0);
    assert!(layout.popup.y >= 0.0);
    assert!(layout.popup.x + layout.popup.w <= context.screen_width);
    assert!(layout.popup.y + layout.popup.h <= context.screen_height);
}
