use engine_core::constants::extensions;
use engine_core::scripting::lua_constants::lua_dirs;

use super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use super::navigation::Navigation;
use super::path_filter::{PathFilter, HIDDEN_DIRS, HIDDEN_EXTENSIONS, HIDDEN_FILENAMES};

#[test]
fn dir_visible_hides_hidden_dirs() {
    for &name in HIDDEN_DIRS {
        assert!(!PathFilter::dir_visible(name), "should hide dir: {name}");
    }
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
