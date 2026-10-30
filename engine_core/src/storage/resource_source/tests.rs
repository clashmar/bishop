use super::ResourceSource;
use crate::constants::paths;
use crate::storage::resource_manifest::{ResourceEntry, ResourceManifest};
use std::collections::HashMap;
use std::path::Path;

fn one_file(path: &str, length: usize) -> ResourceManifest {
    ResourceManifest {
        files: vec![ResourceEntry {
            path: path.to_owned(),
            byte_len: length,
        }],
    }
}

#[test]
fn source_when_declared_resource_is_missing_reports_its_path() {
    let error = ResourceSource::from_prefetched(one_file(paths::GAME_RON, 3), HashMap::new())
        .err()
        .unwrap();

    assert!(error.to_string().contains(paths::GAME_RON));
}

#[test]
fn source_when_declared_resource_length_differs_rejects_it() {
    let mut files = HashMap::new();
    files.insert(paths::GAME_RON.to_owned(), vec![1]);

    let error = ResourceSource::from_prefetched(one_file(paths::GAME_RON, 3), files)
        .err()
        .unwrap();

    assert!(error.to_string().contains(paths::GAME_RON));
    assert!(error.to_string().contains("length"));
}

#[test]
fn source_when_undeclared_resource_is_fetched_rejects_it() {
    let mut files = HashMap::new();
    files.insert(paths::GAME_RON.to_owned(), vec![1]);
    files.insert(paths::ECS_RON.to_owned(), vec![2]);

    let error = ResourceSource::from_prefetched(one_file(paths::GAME_RON, 1), files)
        .err()
        .unwrap();

    assert!(error.to_string().contains(paths::ECS_RON));
}

#[test]
fn manifest_when_entries_are_unsorted_rejects_them() {
    let manifest = ResourceManifest {
        files: vec![
            ResourceEntry { path: paths::GAME_RON.to_owned(), byte_len: 1 },
            ResourceEntry { path: paths::ECS_RON.to_owned(), byte_len: 1 },
        ],
    };

    assert!(manifest.validate().unwrap_err().to_string().contains("sorted"));
}

#[test]
fn manifest_when_entries_are_duplicated_rejects_them() {
    let manifest = ResourceManifest {
        files: vec![
            ResourceEntry { path: paths::GAME_RON.to_owned(), byte_len: 1 },
            ResourceEntry { path: paths::GAME_RON.to_owned(), byte_len: 1 },
        ],
    };

    assert!(manifest.validate().unwrap_err().to_string().contains("duplicate"));
}

#[test]
fn manifest_when_path_escapes_or_is_noncanonical_rejects_it() {
    for path in ["../game.ron", "/game.ron", "./game.ron", "worlds//game.ron", "worlds\\game.ron"] {
        let error = one_file(path, 1).validate().unwrap_err();
        assert!(error.to_string().contains(path), "{error}");
    }
}

#[test]
fn prefetched_files_under_when_nested_lists_only_matching_directory_in_order() {
    let dir = Path::new(paths::PREFABS_FOLDER);
    let paths = [dir.join("z.prefab"), dir.join("nested/a.prefab"),
        Path::new(paths::SCRIPTS_FOLDER).join("ignored.lua")];
    let mut files = HashMap::new();
    let mut manifest = Vec::new();
    for path in paths {
        let key = path.components().map(|part| part.as_os_str().to_str().unwrap())
            .collect::<Vec<_>>().join("/");
        files.insert(key.clone(), vec![1]);
        manifest.push(ResourceEntry { path: key, byte_len: 1 });
    }
    manifest.sort_by(|left, right| left.path.cmp(&right.path));
    let source = ResourceSource::from_prefetched(ResourceManifest { files: manifest }, files).unwrap();

    let found = source.prefetched_files_under(dir).unwrap();

    assert_eq!(found, vec![dir.join("nested/a.prefab"), dir.join("z.prefab")]);
}

#[test]
fn lua_sources_when_prefetched_file_is_invalid_utf8_reports_its_path() {
    let path = Path::new(paths::SCRIPTS_FOLDER).join("bad.lua");
    let key = path.components().map(|component| component.as_os_str().to_str().unwrap())
        .collect::<Vec<_>>().join("/");
    let mut files = HashMap::new();
    files.insert(key.clone(), vec![255]);
    let source = ResourceSource::from_prefetched(one_file(&key, 1), files).unwrap();

    let error = source.prefetched_lua_sources().unwrap_err();

    assert!(error.to_string().contains(&key), "{error}");
}

#[test]
fn source_when_prefetched_reads_declared_bytes_and_reports_missing_lookup() {
    let mut files = HashMap::new();
    files.insert(paths::GAME_RON.to_owned(), vec![1, 2, 3]);
    let source = ResourceSource::from_prefetched(one_file(paths::GAME_RON, 3), files).unwrap();

    assert_eq!(source.read_required(Path::new(paths::GAME_RON)).unwrap(), vec![1, 2, 3]);
    assert!(source.read_optional(Path::new(paths::ECS_RON)).unwrap().is_none());
    assert!(source.read_required(Path::new(paths::ECS_RON)).unwrap_err().to_string().contains(paths::ECS_RON));
}
