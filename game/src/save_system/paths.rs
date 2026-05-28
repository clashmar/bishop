use super::{
    SaveLane, SaveSlotKey, LATEST_RUNTIME_SAVE_MANIFEST, RUNTIME_SAVES_ROOT,
    RUNTIME_SAVE_SLOTS_FOLDER,
};
use engine_core::constants::extensions;
use engine_core::engine_global::{game_name, get_engine_mode};
use engine_core::prelude::EngineMode;
use engine_core::storage::{absolute_save_root, sanitise_name};
use std::path::PathBuf;

/// Engine-managed root for all runtime save files.
pub fn runtime_saves_root() -> PathBuf {
    let root = runtime_save_base_root().join(RUNTIME_SAVES_ROOT);
    if let Some(folder) = runtime_game_folder() {
        root.join(folder)
    } else {
        root
    }
}

fn runtime_save_base_root() -> PathBuf {
    if get_engine_mode() == EngineMode::Game && !cfg!(debug_assertions) {
        return absolute_save_root();
    }

    if cfg!(debug_assertions) || get_engine_mode() == EngineMode::Playtest {
        return absolute_save_root()
            .parent()
            .expect("workspace games root should have a parent directory")
            .to_path_buf();
    }

    absolute_save_root()
}

fn runtime_game_folder() -> Option<String> {
    if get_engine_mode() == EngineMode::Game && !cfg!(debug_assertions) {
        return None;
    }

    let folder = sanitise_name(&game_name());
    Some(if folder.is_empty() {
        "Game".to_string()
    } else {
        folder
    })
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
