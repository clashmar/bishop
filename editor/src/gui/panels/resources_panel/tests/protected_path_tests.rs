use engine_core::constants::paths;
use engine_core::storage::system_folder::{is_protected_path, SYSTEM_FOLDER_ROOTS};

#[test]
fn is_protected_path_detects_system_roots() {
    let root = std::path::Path::new("/games/Demo/Resources");

    for &folder in SYSTEM_FOLDER_ROOTS {
        let path = root.join(folder);
        assert!(is_protected_path(&path, root), "should protect: {folder}");
    }
}

#[test]
fn is_protected_path_exact_match() {
    let root = std::path::Path::new("/games/Demo/Resources");

    assert!(is_protected_path(&root.join(paths::SCRIPTS_FOLDER), root));
    assert!(is_protected_path(
        &root
            .join(paths::TEXT_FOLDER)
            .join(paths::TEXT_LANGUAGE_FOLDER)
            .join(paths::UI_TEXT_FOLDER),
        root
    ));
    assert!(is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join(paths::SFX_FOLDER),
        root
    ));
    assert!(is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join(paths::MUSIC_FOLDER),
        root
    ));
}

#[test]
fn is_protected_path_rejects_user_subdirs_inside_system_roots() {
    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(
        &root.join(paths::SCRIPTS_FOLDER).join("user_subdir"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::ASSETS_FOLDER).join("props"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join("custom"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::TEXT_FOLDER).join("my_text"),
        root
    ));
}

#[test]
fn is_protected_path_rejects_user_dirs() {
    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(&root.join("my_stuff"), root));
    assert!(!is_protected_path(&root.join("user_data"), root));
}

#[test]
fn is_protected_path_rejects_root_itself() {
    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(root, root));
}
