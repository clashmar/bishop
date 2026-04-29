use super::super::*;
use super::*;

use super::super::context_menu::{EntryKind, PendingResourceAction};
use super::super::icon_mapper::IconType;
use engine_core::assets::{AssetKey, AssetRegistry};
use engine_core::ecs::ScriptId;
use engine_core::storage::path_utils::resources_folder_current;

#[test]
fn pending_delete_for_registered_file_returns_delete_registered_file() {
    let (_test_game, _lock) = setup_test_game("pending_delete_registered");
    let mut panel = ResourcesPanel::new();
    let mut registry = AssetRegistry::default();

    let full_path = resources_folder_current().join("scripts/test.lua");
    registry
        .register_asset_relative_path(AssetKey::Script(ScriptId(1)), "test.lua")
        .unwrap();
    registry.init_editor_metadata();

    panel.entries = vec![Entry {
        name: "test.lua".to_string(),
        display_name: "test.lua".to_string(),
        kind: EntryKind::RegisteredFile,
        path: full_path.clone(),
        icon_type: IconType::File,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(matches!(
        action,
        Some(PendingResourceAction::DeleteRegisteredFile(
            AssetKey::Script(ScriptId(1))
        ))
    ));
}

#[test]
fn pending_delete_for_unregistered_file_returns_delete_unregistered_file() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    let path = PathBuf::from("loose.txt");
    panel.entries = vec![Entry {
        name: "loose.txt".to_string(),
        display_name: "loose.txt".to_string(),
        kind: EntryKind::UnregisteredFile,
        path: path.clone(),
        icon_type: IconType::File,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(
        matches!(action, Some(PendingResourceAction::DeleteUnregisteredFile(ref p)) if p == &path)
    );
}

#[test]
fn pending_delete_for_directory_returns_delete_directory() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    let path = PathBuf::from("my_folder");
    panel.entries = vec![Entry {
        name: "my_folder".to_string(),
        display_name: "my_folder".to_string(),
        kind: EntryKind::Directory,
        path: path.clone(),
        icon_type: IconType::Folder,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(
        matches!(action, Some(PendingResourceAction::DeleteDirectory(ref p)) if p.as_ref() == path.as_path())
    );
}

#[test]
fn pending_delete_for_parent_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    panel.selected_indices = [0_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_system_directory_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("assets", EntryKind::SystemDirectory)];
    panel.selected_indices = [0_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_no_selection_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("file.lua", EntryKind::RegisteredFile)];
    panel.selected_indices.clear();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_multi_selection_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![
        test_entry("a.lua", EntryKind::RegisteredFile),
        test_entry("b.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize, 1_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}
