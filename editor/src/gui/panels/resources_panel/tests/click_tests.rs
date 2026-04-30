use super::super::*;
use super::*;

use super::super::context_menu::EntryKind;
use std::collections::BTreeSet;

#[test]
fn single_click_directory_selects_without_navigation() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.contains(&0));
    assert!(panel.navigation.is_at_root());
}

#[test]
fn single_click_parent_is_ignored() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
    assert_eq!(panel.navigation.depth(), 1);
}

#[test]
fn single_click_file_selects_without_navigation() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.contains(&0));
    assert!(panel.navigation.is_at_root());
}

#[test]
fn double_click_directory_navigates_and_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert_eq!(panel.navigation.depth(), 1);
    assert_eq!(panel.navigation.segment(0), Some("subdir"));
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn double_click_parent_navigates_up_and_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert!(panel.navigation.is_at_root());
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn left_click_background_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [2_usize].into_iter().collect();

    panel.clear_selection();

    assert!(panel.selected_indices.is_empty());
}

#[test]
fn resources_panel_multi_select_plain_click_replaces_previous_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("scripts", EntryKind::Directory),
        test_entry("player.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(1, false, false);

    assert!(opened_path.is_none());
    assert_eq!(
        panel.selected_indices,
        [1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_click_adds_unselected_entry() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("scripts", EntryKind::Directory),
        test_entry("player.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(1, true, false);

    assert!(opened_path.is_none());
    assert_eq!(
        panel.selected_indices,
        [0_usize, 1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_click_toggles_selected_entry_off() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, true, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn resources_panel_multi_select_plain_marquee_replaces_selection() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), false);
    panel.commit_marquee_selection([1_usize, 3_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [1_usize, 3_usize].into_iter().collect::<BTreeSet<_>>()
    );
    assert!(!panel.marquee_selection.active);
    assert!(panel.marquee_selection.start_content_pos.is_none());
}

#[test]
fn resources_panel_multi_select_shift_marquee_toggles_matched_entries_off() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [0_usize, 4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), true);
    panel.selected_indices.clear();
    panel.commit_marquee_selection([0_usize, 1_usize, 4_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_marquee_adds_new_entries() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [0_usize, 4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), true);
    panel.selected_indices.clear();
    panel.commit_marquee_selection([1_usize, 3_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [0_usize, 1_usize, 3_usize, 4_usize]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_clear_selection_does_not_drop_marquee_snapshot() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [2_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(5.0, 6.0), true);
    panel.clear_selection();
    panel.commit_marquee_selection(BTreeSet::new());

    assert_eq!(
        panel.selected_indices,
        [2_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_double_click_directory_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];
    panel.selected_indices = [0_usize, 3_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
    assert_eq!(panel.navigation.segment(0), Some("subdir"));
}
