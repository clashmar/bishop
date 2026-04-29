use super::super::path_filter::{PathFilter, HIDDEN_DIRS, HIDDEN_EXTENSIONS, HIDDEN_FILENAMES};
use engine_core::constants::paths;
use engine_core::scripting::lua_constants::lua_dirs;

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
fn file_visible_hides_dotfiles() {
    assert!(!PathFilter::file_visible(".DS_Store"));
    assert!(!PathFilter::file_visible(".gitkeep"));
    assert!(!PathFilter::file_visible(".hidden"));
}
