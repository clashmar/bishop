use super::*;
use crate::widgets::test_support::WidgetTestContext;

#[test]
fn suppressed_dropdown_trigger_does_not_open() {
    reset_click_consumed();

    let id = WidgetId(77);
    dropdown_state::set(id, dropdown_state::DropState::default());
    let rect = Rect::new(0.0, 0.0, 80.0, 30.0);
    let options = ["One", "Two"];

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (40.0, 20.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;
    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string())
            .suppressed(true)
            .show(&mut press_ctx),
        None
    );

    reset_click_consumed();
    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (40.0, 20.0);
    release_ctx.left_released = true;
    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string())
            .suppressed(true)
            .show(&mut release_ctx),
        None
    );

    assert!(!dropdown_state::get(id).open);
}

#[test]
fn suppressed_dropdown_closes_existing_open_list() {
    reset_click_consumed();

    let id = WidgetId(78);
    let rect = Rect::new(0.0, 0.0, 80.0, 30.0);
    let list_rect = Rect::new(0.0, 30.0, 80.0, 60.0);
    let options = ["One", "Two"];

    dropdown_state::set(
        id,
        dropdown_state::DropState {
            open: true,
            rect: list_rect,
            scroll_offset: 0.0,
        },
    );

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (40.0, 40.0);
    ctx.left_pressed = true;

    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string())
            .suppressed(true)
            .show(&mut ctx),
        None
    );
    assert!(!dropdown_state::get(id).open);
}
