use super::super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use engine_core::constants::{extensions, paths};
use std::path::PathBuf;

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
fn dir_icon_for_system_root_returns_system_folder() {
    let resources = PathBuf::from("/game/Resources");
    let assets = resources.join(paths::ASSETS_FOLDER);

    assert_eq!(
        IconMapper::dir_icon_for(&assets, &resources),
        IconType::SystemFolder
    );
}

#[test]
fn dir_icon_for_user_folder_returns_folder() {
    let resources = PathBuf::from("/game/Resources");
    let user_dir = resources.join("MyCustomFolder");

    assert_eq!(
        IconMapper::dir_icon_for(&user_dir, &resources),
        IconType::Folder
    );
}

#[test]
fn dir_icon_for_nested_system_subfolder_returns_system_folder() {
    let resources = PathBuf::from("/game/Resources");
    let sfx = resources.join(paths::AUDIO_FOLDER).join(paths::SFX_FOLDER);

    assert_eq!(
        IconMapper::dir_icon_for(&sfx, &resources),
        IconType::SystemFolder
    );
}
