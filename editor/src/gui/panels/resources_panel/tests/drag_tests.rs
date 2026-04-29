use super::*;

#[test]
fn parent_entry_is_not_draggable() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    assert!(!panel.is_draggable(0));
}

#[test]
fn system_directory_is_not_draggable() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("assets", EntryKind::SystemDirectory)];
    assert!(!panel.is_draggable(0));
}

#[test]
fn registered_file_is_draggable() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];
    assert!(panel.is_draggable(0));
}

#[test]
fn build_drag_payload_uses_full_selection_when_pressing_selected_item() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("a.lua", EntryKind::RegisteredFile),
        test_entry("b.lua", EntryKind::RegisteredFile),
        test_entry("c.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0, 2].into_iter().collect();

    let payload = panel.build_drag_payload(0);
    assert_eq!(payload.len(), 2);
    assert!(payload.iter().any(|p| p.name == "a.lua"));
    assert!(payload.iter().any(|p| p.name == "c.lua"));
}

#[test]
fn build_drag_payload_replaces_selection_when_pressing_unselected_item() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("a.lua", EntryKind::RegisteredFile),
        test_entry("b.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0].into_iter().collect();

    let payload = panel.build_drag_payload(1);
    assert_eq!(payload.len(), 1);
    assert_eq!(payload[0].name, "b.lua");
}

#[test]
fn pressing_selected_item_preserves_multi_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("a.lua", EntryKind::RegisteredFile),
        test_entry("b.lua", EntryKind::RegisteredFile),
        test_entry("c.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0, 2].into_iter().collect();

    // When pressing on an already-selected item, the panel should not
    // call handle_primary_click_on_entry (which would clear the selection).
    // The payload should include all selected items and selection preserved.
    let payload = panel.build_drag_payload(0);

    assert_eq!(panel.selected_indices.len(), 2);
    assert!(panel.selected_indices.contains(&0));
    assert!(panel.selected_indices.contains(&2));
    assert_eq!(payload.len(), 2);
}

#[test]
fn drop_target_index_finds_directory_under_cursor() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("file.lua", EntryKind::RegisteredFile),
        test_entry("subdir", EntryKind::Directory),
    ];
    let content_rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let cols = 2;
    let cell = cell_content_rect(1, cols);
    let mouse = Vec2::new(
        content_rect.x + cell.x + cell.w / 2.0,
        content_rect.y + cell.y + cell.h / 2.0,
    );
    let result = panel.drop_target_index(mouse, content_rect, cols);
    assert_eq!(result, Some(1));
}

#[test]
fn drop_target_index_returns_none_for_file() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("file.lua", EntryKind::RegisteredFile),
        test_entry("subdir", EntryKind::Directory),
    ];
    let content_rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let cols = 2;
    let cell = cell_content_rect(0, cols);
    let mouse = Vec2::new(
        content_rect.x + cell.x + cell.w / 2.0,
        content_rect.y + cell.y + cell.h / 2.0,
    );
    let result = panel.drop_target_index(mouse, content_rect, cols);
    assert_eq!(result, None);
}

#[test]
fn drop_target_index_returns_none_for_dragged_item() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("file.lua", EntryKind::RegisteredFile),
        test_entry("subdir", EntryKind::Directory),
    ];
    panel.drag_state.payload = vec![DragPayload {
        path: PathBuf::from("subdir"),
        name: "subdir".to_string(),
        icon_type: IconType::Folder,
    }];
    let content_rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let cols = 2;
    let cell = cell_content_rect(1, cols);
    let mouse = Vec2::new(
        content_rect.x + cell.x + cell.w / 2.0,
        content_rect.y + cell.y + cell.h / 2.0,
    );
    let result = panel.drop_target_index(mouse, content_rect, cols);
    assert_eq!(result, None);
}
