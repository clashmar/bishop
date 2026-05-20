use super::{SaveLane, SaveSlotKey};
use crate::save_system::constants::{
    LATEST_RUNTIME_SAVE_MANIFEST, RUNTIME_SAVES_ROOT, RUNTIME_SAVE_SLOTS_FOLDER,
};
use engine_core::constants::extensions;
use engine_core::storage::absolute_save_root;
use std::path::PathBuf;

/// Engine-managed root for all runtime save files.
pub fn runtime_saves_root() -> PathBuf {
    absolute_save_root().join(RUNTIME_SAVES_ROOT)
}

/// Slot directory root.
pub fn runtime_slots_root() -> PathBuf {
    runtime_saves_root().join(RUNTIME_SAVE_SLOTS_FOLDER)
}

/// Directory for a specific save slot.
pub fn runtime_slot_folder(slot: &SaveSlotKey) -> PathBuf {
    runtime_slots_root().join(slot.folder_name())
}

/// Save file path for a lane in a specific slot.
pub fn runtime_save_file(slot: &SaveSlotKey, lane: SaveLane) -> PathBuf {
    runtime_slot_folder(slot).join(format!("{}.{}", lane.file_stem(), extensions::RON))
}

/// Latest-save manifest path for the current game.
pub fn runtime_latest_save_manifest_path() -> PathBuf {
    runtime_saves_root().join(LATEST_RUNTIME_SAVE_MANIFEST)
}
