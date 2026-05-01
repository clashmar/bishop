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

#[test]
fn close_open_dropdowns_clears_filterable_state() {
    reset_click_consumed();

    let id = WidgetId(90);
    let rect = Rect::new(0.0, 0.0, 120.0, 30.0);
    let list_rect = Rect::new(0.0, 30.0, 120.0, 90.0);
    let filter_id = WidgetId(id.0.wrapping_add(FILTER_ID_OFFSET));
    let options = ["Alpha", "Beta"];

    let mut ctx = WidgetTestContext::new();
    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string())
            .filterable()
            .show(&mut ctx),
        None
    );

    dropdown_state::set(
        id,
        dropdown_state::DropState {
            open: true,
            rect: list_rect,
            scroll_offset: 0.0,
        },
    );
    set_filter(id, "alp".to_string());
    INPUT_TEXT_STATE.with(|state| {
        let mut input_state = TextInputState::new("alp".to_string());
        input_state.focused = true;
        state.borrow_mut().insert(filter_id, input_state);
    });
    request_focus(filter_id, true);
    update_global_dropdown_flag();

    close_open_dropdowns();

    assert!(!dropdown_state::get(id).open);
    assert!(!is_dropdown_open());
    assert_eq!(get_filter(id), "");
    assert!(!input_is_focused());

    INPUT_TEXT_STATE.with(|state| {
        assert!(!state.borrow().contains_key(&filter_id));
    });
}

#[test]
fn empty_non_filterable_dropdown_does_not_open() {
    reset_click_consumed();

    let id = WidgetId(91);
    let rect = Rect::new(0.0, 0.0, 120.0, 30.0);
    let options: [&str; 0] = [];

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (60.0, 15.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string()).show(&mut press_ctx),
        None
    );

    reset_click_consumed();
    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (60.0, 15.0);
    release_ctx.left_released = true;

    assert_eq!(
        Dropdown::new(id, rect, "Pick", &options, |opt| opt.to_string()).show(&mut release_ctx),
        None
    );
    assert!(!dropdown_state::get(id).open);
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::{Theme, WidgetThemeMapper};

    #[test]
    fn dropdown_theme_mapper_maps_key_roles() {
        let theme = Theme {
            surface: Color::GREEN,
            text: Color::BLUE,
            border: Color::new(0.8, 0.8, 0.8, 1.0),
            hover: Color::new(0.2, 0.2, 1.0, 1.0),
            ..Theme::default()
        };
        let visuals = Dropdown::<&str>::theme_visuals(&theme);
        assert_eq!(visuals.background, Some(Color::GREEN));
        assert_eq!(visuals.text, Some(Color::BLUE));
        assert_eq!(visuals.border, Some(Color::new(0.8, 0.8, 0.8, 1.0)));
        assert_eq!(visuals.hover, Some(Color::new(0.2, 0.2, 1.0, 1.0)));
        assert_eq!(visuals.primary, None);
        assert_eq!(visuals.surface, None);
        assert_eq!(visuals.accent, None);
    }
}
