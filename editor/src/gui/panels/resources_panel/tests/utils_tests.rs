use super::super::*;

#[test]
fn resources_panel_multi_select_content_space_position_accounts_for_scroll() {
    let content_rect = Rect::new(100.0, 200.0, 300.0, 400.0);
    let mouse = Vec2::new(160.0, 260.0);

    let pos = content_space_mouse_position(mouse, content_rect, -72.0);

    assert_eq!(pos, Vec2::new(60.0, 132.0));
}
