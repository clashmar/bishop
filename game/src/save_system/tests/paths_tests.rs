use super::RuntimeSaveTestContext;
use engine_core::constants::extensions;
use engine_core::storage::{absolute_save_root, resources_folder, sanitise_name};

use crate::save_system::{
    runtime_latest_save_manifest_path,
    runtime_save_file,
    runtime_saves_root,
    runtime_slot_folder,
    SaveLane,
    SaveSlotKey,
    RUNTIME_SAVE_SLOTS_FOLDER,
    DEFAULT_RUNTIME_SAVE_SLOT,
    LATEST_RUNTIME_SAVE_MANIFEST,
    RUNTIME_SAVES_ROOT,
};

#[test]
fn runtime_save_root_lives_outside_resources_folder() {
    let ctx = RuntimeSaveTestContext::new("runtime_save_root");
    let root = runtime_saves_root();
    let resources = resources_folder(ctx.game_name());

    assert!(!root.starts_with(&resources));
}

#[test]
fn runtime_save_root_moves_to_workspace_root_and_namespaces_by_game() {
    let ctx = RuntimeSaveTestContext::new("runtime_save_workspace_root");

    assert_eq!(
        runtime_saves_root(),
        absolute_save_root()
            .parent()
            .expect("debug game save root should be nested under the workspace root")
            .join(RUNTIME_SAVES_ROOT)
            .join(sanitise_name(ctx.game_name()))
    );
}

#[test]
fn runtime_save_file_uses_default_slot_and_lane_name() {
    let _ctx = RuntimeSaveTestContext::new("runtime_save_file_default");
    let slot = SaveSlotKey::Default;

    assert_eq!(
        runtime_save_file(&slot, SaveLane::Manual),
        runtime_saves_root()
            .join(RUNTIME_SAVE_SLOTS_FOLDER)
            .join(DEFAULT_RUNTIME_SAVE_SLOT)
            .join(format!("manual.{}", extensions::RON))
    );

    assert_eq!(
        runtime_save_file(&slot, SaveLane::Autosave),
        runtime_saves_root()
            .join(RUNTIME_SAVE_SLOTS_FOLDER)
            .join(DEFAULT_RUNTIME_SAVE_SLOT)
            .join(format!("autosave.{}", extensions::RON))
    );
}

#[test]
fn runtime_slot_folder_sanitises_named_slots() {
    let _ctx = RuntimeSaveTestContext::new("runtime_named_slot");
    let slot = SaveSlotKey::Named("Boss Rush!!!".to_string());

    assert_eq!(
        runtime_slot_folder(&slot)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("Boss Rush")
    );
}

#[test]
fn latest_manifest_path_lives_at_runtime_root() {
    let _ctx = RuntimeSaveTestContext::new("runtime_latest_manifest");

    assert_eq!(
        runtime_latest_save_manifest_path(),
        runtime_saves_root().join(LATEST_RUNTIME_SAVE_MANIFEST)
    );
}
