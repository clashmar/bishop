use engine_core::assets::AssetRegistry;
use engine_core::constants::{extensions, paths};
use engine_core::engine_global::set_game_name;
use engine_core::scripting::lua_constants::lua_dirs;
use engine_core::storage::path_utils::resources_folder_current;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

use super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use super::navigation::Navigation;
use super::path_filter::{PathFilter, HIDDEN_DIRS, HIDDEN_EXTENSIONS, HIDDEN_FILENAMES};
use super::ResourcesPanel;

#[test]
fn dir_visible_hides_hidden_dirs() {
    for &name in HIDDEN_DIRS {
        assert!(!PathFilter::dir_visible(name), "should hide dir: {name}");
    }
}

#[test]
fn file_visible_hides_language_manifest() {
    assert!(!PathFilter::file_visible(paths::LANGUAGE_MANIFEST));
}

#[test]
fn dir_visible_hides_engine_dir() {
    assert!(!PathFilter::dir_visible(lua_dirs::ENGINE));
}

#[test]
fn dir_visible_allows_unknown() {
    assert!(PathFilter::dir_visible("my_custom_dir"));
}

#[test]
fn file_visible_hides_hidden_filenames() {
    for &name in HIDDEN_FILENAMES {
        assert!(!PathFilter::file_visible(name), "should hide file: {name}");
    }
}

#[test]
fn file_visible_hides_hidden_extensions() {
    for &ext in HIDDEN_EXTENSIONS {
        let filename = format!("test_file.{ext}");
        assert!(
            !PathFilter::file_visible(&filename),
            "should hide .{ext} file"
        );
    }
}

#[test]
fn file_visible_allows_unknown_extension() {
    assert!(PathFilter::file_visible("readme.txt"));
}

#[test]
fn dir_icon_returns_folder() {
    assert_eq!(IconMapper::dir_icon(), IconType::Folder);
}

#[test]
fn file_icon_maps_known_extensions() {
    for &(ext, expected) in FILE_ICON_MAP {
        let filename = format!("test_file.{ext}");
        assert_eq!(
            IconMapper::file_icon(&filename),
            expected,
            "file_icon(.{ext})"
        );
    }
}

#[test]
fn file_icon_unknown_extension_gets_file() {
    assert_eq!(IconMapper::file_icon("data.dat"), IconType::File);
}

#[test]
fn file_icon_no_extension_gets_file() {
    assert_eq!(IconMapper::file_icon("Makefile"), IconType::File);
}

#[test]
fn file_icon_maps_prefab_extension_to_prefab() {
    let filename = format!("test_file.{}", extensions::PREFAB);

    assert_eq!(IconMapper::file_icon(&filename), IconType::Prefab);
}

#[test]
fn file_icon_maps_ron_extension_to_file() {
    let filename = format!("test_file.{}", extensions::RON);

    assert_eq!(IconMapper::file_icon(&filename), IconType::File);
}

#[test]
fn navigation_starts_at_root() {
    let nav = Navigation::new();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_push_goes_into_subdirectory() {
    let mut nav = Navigation::new();
    nav.push("assets");
    assert!(!nav.is_at_root());
}

#[test]
fn navigation_pop_goes_back_to_parent() {
    let mut nav = Navigation::new();
    nav.push("assets");
    let went_back = nav.pop();
    assert!(went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_pop_at_root_returns_false() {
    let mut nav = Navigation::new();
    let went_back = nav.pop();
    assert!(!went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_deep_path_push_pop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_depth_reflects_segment_count() {
    let mut nav = Navigation::new();
    assert_eq!(nav.depth(), 0);
    nav.push("assets");
    assert_eq!(nav.depth(), 1);
    nav.push("sprites");
    assert_eq!(nav.depth(), 2);
    nav.pop();
    assert_eq!(nav.depth(), 1);
}

#[test]
fn navigation_truncate_to_root() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(0);
    assert!(nav.is_at_root());
    assert_eq!(nav.depth(), 0);
}

#[test]
fn navigation_truncate_to_mid_depth() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(1);
    assert_eq!(nav.depth(), 1);
    assert_eq!(nav.segment(0), Some("assets"));
}

#[test]
fn navigation_truncate_to_current_depth_is_noop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.truncate_to(2);
    assert_eq!(nav.depth(), 2);
}

#[test]
fn navigation_segment_returns_correct_value() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    assert_eq!(nav.segment(0), Some("assets"));
    assert_eq!(nav.segment(1), Some("sprites"));
    assert_eq!(nav.segment(2), None);
}

fn setup_test_game(test_prefix: &str) -> (TestGameFolder, impl Drop) {
    let lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new(test_prefix);
    set_game_name(test_game.name());
    let resources = resources_folder_current();
    std::fs::create_dir_all(resources.join("subdir")).unwrap();
    std::fs::create_dir_all(resources.join("subdir").join("nested")).unwrap();
    std::fs::write(resources.join("subdir").join("test.lua"), "").unwrap();
    (test_game, lock)
}

#[test]
fn scan_at_root_has_no_parent_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_no_parent_at_root");
    let mut panel = ResourcesPanel::new();
    panel.scan_current_dir(&AssetRegistry::default());
    assert!(panel.navigation.is_at_root());
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent));
}

#[test]
fn scan_in_subdir_has_parent_as_first_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_first");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have a parent entry");
    assert!(first.is_parent);
    assert_eq!(first.display_name, "..");
    assert!(first.is_dir);
    assert!(!first.is_registered);
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
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent));
}

#[test]
fn parent_entry_appears_at_each_depth() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_each_depth");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.navigation.push("nested");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent);

    panel.navigation.pop();
    panel.scan_current_dir(&AssetRegistry::default());
    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent);
}
