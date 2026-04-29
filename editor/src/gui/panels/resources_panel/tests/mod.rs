mod click_tests;
mod context_menu_tests;
mod icon_mapper_tests;
mod navigation_tests;
mod open_resource_tests;
mod path_filter_tests;
mod pending_delete_tests;
mod protected_path_tests;
mod scan_tests;
mod utils_tests;

pub use super::*;

use super::context_menu::EntryKind;
use super::icon_mapper::IconType;
use crate::test_utils::game_fs_test_lock;
use engine_core::engine_global::set_game_name;
use engine_core::storage::path_utils::resources_folder_current;
use std::path::PathBuf;

pub fn test_entry(name: &str, kind: EntryKind) -> Entry {
    Entry {
        name: name.to_string(),
        display_name: name.to_string(),
        kind,
        path: PathBuf::from(name),
        icon_type: if matches!(
            kind,
            EntryKind::Parent | EntryKind::Directory | EntryKind::SystemDirectory
        ) {
            IconType::Folder
        } else {
            IconType::File
        },
    }
}

pub fn setup_test_game(test_prefix: &str) -> (crate::test_utils::TestGameFolder, impl Drop) {
    let lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = crate::test_utils::TestGameFolder::new(test_prefix);
    set_game_name(test_game.name());
    let resources = resources_folder_current();
    std::fs::create_dir_all(resources.join("subdir")).unwrap();
    std::fs::create_dir_all(resources.join("subdir").join("nested")).unwrap();
    std::fs::write(resources.join("subdir").join("test.lua"), "").unwrap();
    (test_game, lock)
}
