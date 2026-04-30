use super::super::*;
use super::*;

use engine_core::assets::AssetRegistry;

#[test]
fn scan_at_root_has_no_parent_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_no_parent_at_root");
    let mut panel = ResourcesPanel::new();
    panel.scan_current_dir(&AssetRegistry::default());
    assert!(panel.navigation.is_at_root());
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent()));
}

#[test]
fn scan_in_subdir_has_parent_as_first_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_first");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have a parent entry");
    assert!(first.is_parent());
    assert_eq!(first.display_name, "..");
    assert!(first.is_dir_like());
    assert!(!first.is_registered());
}

#[test]
fn clicking_parent_entry_navigates_to_root() {
    let (_test_game, _lock) = setup_test_game("resources_panel_click_parent");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.scan_current_dir(&AssetRegistry::default());

    assert!(!panel.navigation.is_at_root());
    panel.navigation.pop();
    panel.scan_current_dir(&AssetRegistry::default());
    assert!(panel.navigation.is_at_root());
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent()));
}

#[test]
fn parent_entry_appears_at_each_depth() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_each_depth");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.navigation.push("nested");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent());

    panel.navigation.pop();
    panel.scan_current_dir(&AssetRegistry::default());
    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent());
}
