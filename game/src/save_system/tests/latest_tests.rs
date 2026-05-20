use super::CleanSaveRoot;
use std::io::ErrorKind;

use crate::save_system::{
    runtime_latest_save_manifest_path, LatestRuntimeSaveManifest, RonPersist, SaveLane,
    SaveSlotKey,
};

#[test]
fn latest_manifest_round_trips_through_disk() {
    let _cleanup = CleanSaveRoot::new();
    let path = runtime_latest_save_manifest_path();
    let manifest = LatestRuntimeSaveManifest {
        lane: SaveLane::Autosave,
        slot: SaveSlotKey::Default,
        saved_at_unix_ms: 456,
    };

    manifest.write_to_path(&path).unwrap();
    let loaded = LatestRuntimeSaveManifest::read_from_path(&path).unwrap();

    assert_eq!(loaded, manifest);
}

#[test]
fn latest_manifest_missing_file_returns_not_found() {
    let _cleanup = CleanSaveRoot::new();
    let path = runtime_latest_save_manifest_path();

    let error = LatestRuntimeSaveManifest::read_from_path(&path).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}
