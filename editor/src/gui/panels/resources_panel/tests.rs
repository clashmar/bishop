use engine_core::constants::paths;
use engine_core::scripting::lua_constants::lua_dirs;
use std::path::PathBuf;

use super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use super::navigation::Navigation;
use super::path_filter::{PathFilter, HIDDEN_DIRS, HIDDEN_EXTENSIONS, HIDDEN_FILENAMES};

fn test_root() -> PathBuf {
    PathBuf::from("/games/Demo").join(paths::RESOURCES_FOLDER)
}

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
fn navigation_starts_at_root() {
    let nav = Navigation::new(test_root());
    assert!(nav.is_at_root());
    assert_eq!(nav.current(), test_root().as_path());
}

#[test]
fn navigation_push_goes_into_subdirectory() {
    let mut nav = Navigation::new(test_root());
    nav.push(paths::ASSETS_FOLDER);
    assert!(!nav.is_at_root());
    assert_eq!(
        nav.current(),
        test_root().join(paths::ASSETS_FOLDER).as_path()
    );
}

#[test]
fn navigation_pop_goes_back_to_parent() {
    let mut nav = Navigation::new(test_root());
    nav.push(paths::ASSETS_FOLDER);
    let went_back = nav.pop();
    assert!(went_back);
    assert!(nav.is_at_root());
    assert_eq!(nav.current(), test_root().as_path());
}

#[test]
fn navigation_pop_at_root_returns_false() {
    let mut nav = Navigation::new(test_root());
    let went_back = nav.pop();
    assert!(!went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_deep_path_push_pop() {
    let mut nav = Navigation::new(test_root());
    nav.push(paths::ASSETS_FOLDER);
    nav.push("sprites");
    nav.push("tiles");
    let expected = test_root()
        .join(paths::ASSETS_FOLDER)
        .join("sprites")
        .join("tiles");
    assert_eq!(nav.current(), expected.as_path());
    nav.pop();
    assert_eq!(
        nav.current(),
        test_root()
            .join(paths::ASSETS_FOLDER)
            .join("sprites")
            .as_path()
    );
    nav.pop();
    assert_eq!(
        nav.current(),
        test_root().join(paths::ASSETS_FOLDER).as_path()
    );
    nav.pop();
    assert!(nav.is_at_root());
}
