use super::super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use engine_core::constants::extensions;

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
