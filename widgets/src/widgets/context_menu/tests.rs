use super::*;
use crate::set_modal_open;
use crate::widgets::test_support::WidgetTestContext;

fn make_items() -> Vec<ContextMenuItem<String>> {
    vec![
        ContextMenuItem {
            label: "Rename".to_string(),
            value: "rename".to_string(),
        },
        ContextMenuItem {
            label: "Delete".to_string(),
            value: "delete".to_string(),
        },
        ContextMenuItem {
            label: "Reveal".to_string(),
            value: "reveal".to_string(),
        },
    ]
}

#[test]
fn empty_items_does_not_open() {
    reset_click_consumed();

    let id = WidgetId(400);
    let items: Vec<ContextMenuItem<String>> = Vec::new();

    let mut ctx = WidgetTestContext::new();
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(10.0, 10.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
}

#[test]
fn right_click_opens_menu() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let id = WidgetId(401);
    let items = make_items();

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (50.0, 50.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    assert!(context_menu_state::get(id).open);
    assert!(is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn just_opened_prevents_selection_on_open_frame() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let id = WidgetId(402);
    let items = make_items();

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (55.0, 55.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    let state = context_menu_state::get(id);
    assert!(state.open);
    assert!(!state.just_opened);

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn clicking_item_returns_value_and_closes() {
    let id = WidgetId(403);
    let items = make_items();
    let rect = Rect::new(50.0, 50.0, 100.0, 90.0);

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect,
            just_opened: false,
        },
    );
    set_context_menu_open(true);

    reset_click_consumed();
    clear_click_target(MouseButton::Left);
    clear_click_target(MouseButton::Right);

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (55.0, 65.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut press_ctx);
    assert_eq!(result, None);

    reset_click_consumed();

    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (55.0, 65.0);
    release_ctx.left_released = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut release_ctx);
    assert_eq!(result, Some(items[0].value.clone()));
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn clicking_outside_dismisses_menu() {
    let id = WidgetId(404);
    let items = make_items();

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect: Rect::new(50.0, 50.0, 100.0, 90.0),
            just_opened: false,
        },
    );
    set_context_menu_open(true);

    reset_click_consumed();
    clear_click_target(MouseButton::Left);
    clear_click_target(MouseButton::Right);

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (300.0, 200.0);
    ctx.left_pressed = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn blocked_context_menu_does_not_open() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let id = WidgetId(405);
    let items = make_items();

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (50.0, 50.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items)
        .blocked(true)
        .show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn suppressed_context_menu_does_not_open() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let id = WidgetId(406);
    let items = make_items();

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (50.0, 50.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items)
        .suppressed(true)
        .show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
}

#[test]
fn opening_context_menu_closes_dropdowns() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let dropdown_id = WidgetId(500);
    let context_id = WidgetId(407);
    let items = make_items();

    crate::widgets::dropdown::dropdown_state::set(
        dropdown_id,
        crate::widgets::dropdown::dropdown_state::DropState {
            open: true,
            rect: Rect::new(0.0, 0.0, 80.0, 60.0),
            scroll_offset: 0.0,
        },
    );
    crate::widgets::dropdown::update_global_dropdown_flag();
    assert!(is_dropdown_open());

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (50.0, 50.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let _ = ContextMenu::new(context_id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert!(!is_dropdown_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&context_id));
    crate::widgets::dropdown::dropdown_state::STATE.with(|s| s.borrow_mut().remove(&dropdown_id));
    set_context_menu_open(false);
}

#[test]
fn close_open_context_menus_clears_state() {
    let id = WidgetId(408);

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect: Rect::new(50.0, 50.0, 100.0, 90.0),
            just_opened: false,
        },
    );
    set_context_menu_open(true);
    assert!(is_context_menu_open());

    close_open_context_menus();

    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
}

#[test]
fn right_click_outside_closes_menu() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);

    let id = WidgetId(409);
    let items = make_items();

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect: Rect::new(50.0, 50.0, 100.0, 90.0),
            just_opened: false,
        },
    );
    set_context_menu_open(true);

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (300.0, 200.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
}

#[test]
fn is_context_menu_open_reflects_state() {
    let id = WidgetId(410);

    assert!(!is_context_menu_open());

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect: Rect::new(0.0, 0.0, 100.0, 60.0),
            just_opened: false,
        },
    );
    set_context_menu_open(true);
    assert!(is_context_menu_open());

    close_open_context_menus();
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
}

#[test]
fn modal_open_prevents_context_menu_open() {
    reset_click_consumed();
    clear_click_target(MouseButton::Right);
    clear_click_target(MouseButton::Left);
    set_modal_open(true);

    let id = WidgetId(411);
    let items = make_items();

    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (50.0, 50.0);
    ctx.right_pressed = true;
    ctx.right_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut ctx);
    assert_eq!(result, None);
    assert!(!context_menu_state::get(id).open);
    assert!(!is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
    set_modal_open(false);
}

#[test]
fn modal_open_prevents_item_selection() {
    let id = WidgetId(412);
    let items = make_items();
    let rect = Rect::new(50.0, 50.0, 100.0, 90.0);

    context_menu_state::set(
        id,
        context_menu_state::ContextMenuState {
            open: true,
            rect,
            just_opened: false,
        },
    );
    set_context_menu_open(true);
    set_modal_open(true);

    reset_click_consumed();
    clear_click_target(MouseButton::Left);
    clear_click_target(MouseButton::Right);

    let mut press_ctx = WidgetTestContext::new();
    press_ctx.mouse_pos = (55.0, 65.0);
    press_ctx.left_pressed = true;
    press_ctx.left_down = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut press_ctx);
    assert_eq!(result, None);

    reset_click_consumed();

    let mut release_ctx = WidgetTestContext::new();
    release_ctx.mouse_pos = (55.0, 65.0);
    release_ctx.left_released = true;

    let result = ContextMenu::new(id, Vec2::new(50.0, 50.0), &items).show(&mut release_ctx);
    assert_eq!(result, None);
    assert!(context_menu_state::get(id).open);
    assert!(is_context_menu_open());

    context_menu_state::STATE.with(|s| s.borrow_mut().remove(&id));
    set_context_menu_open(false);
    set_modal_open(false);
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn context_menu_theme_mapper_maps_key_roles() {
        let theme = Theme {
            surface: Color::GREEN,
            text: Color::BLUE,
            border: Color::new(0.8, 0.8, 0.8, 1.0),
            hover: Color::new(0.2, 0.2, 1.0, 1.0),
            ..Theme::default()
        };
        let overrides = ContextMenu::<String>::map_theme(&theme);
        assert_eq!(overrides.background, Some(Color::GREEN));
        assert_eq!(overrides.text, Some(Color::BLUE));
        assert_eq!(overrides.border, Some(Color::new(0.8, 0.8, 0.8, 1.0)));
        assert_eq!(overrides.hover, Some(Color::new(0.2, 0.2, 1.0, 1.0)));
        assert_eq!(overrides.primary, None);
        assert_eq!(overrides.surface, None);
        assert_eq!(overrides.accent, None);
    }
}
