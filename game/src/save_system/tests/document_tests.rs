use super::RuntimeSaveTestContext;
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

use crate::save_system::{
    runtime_save_file,
    runtime_saves_root,
    RuntimeSaveDocument,
    RuntimeSaveMetadata,
    RUNTIME_SAVE_SCHEMA_VERSION,
    SavedSection,
    SaveLane,
    SaveSlotKey,
};

// --- Helpers ---

fn sample_document(game_name: &str) -> RuntimeSaveDocument {
    let mut sections = HashMap::new();
    sections.insert(
        "game.player".to_string(),
        SavedSection {
            version: 1,
            data: "(position:(x:12.0,y:8.0))".to_string(),
        },
    );
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "(room_id:4)".to_string(),
        },
    );

    RuntimeSaveDocument {
        metadata: RuntimeSaveMetadata {
            schema_version: RUNTIME_SAVE_SCHEMA_VERSION,
            game_id: Uuid::nil(),
            game_name: game_name.to_string(),
            lane: SaveLane::Manual,
            slot: SaveSlotKey::Default,
            saved_at_unix_ms: 123,
        },
        sections,
    }
}

// --- Tests ---

#[test]
fn runtime_save_document_round_trips_through_ron() {
    let ctx = RuntimeSaveTestContext::new("runtime_document_round_trip");
    let document = sample_document(ctx.game_name());
    let ron = document.to_ron_string().unwrap();
    let reparsed = RuntimeSaveDocument::from_ron_str(&ron).unwrap();

    assert_eq!(reparsed, document);
}

#[test]
fn runtime_save_document_serializes_sections_in_key_order() {
    let ctx = RuntimeSaveTestContext::new("runtime_document_order");
    let document = sample_document(ctx.game_name());
    let ron = document.to_ron_string().unwrap();

    let engine_index = ron.find("engine.resume").unwrap();
    let player_index = ron.find("game.player").unwrap();
    assert!(engine_index < player_index);
}

#[test]
fn write_to_path_creates_parent_directories() {
    let ctx = RuntimeSaveTestContext::new("runtime_document_write");
    let path = runtime_save_file(&SaveSlotKey::Default, SaveLane::Manual);
    let document = sample_document(ctx.game_name());

    let _ = fs::remove_dir_all(runtime_saves_root());
    document.write_to_path(&path).unwrap();

    assert!(path.exists());
    let loaded = RuntimeSaveDocument::read_from_path(&path).unwrap();
    assert_eq!(loaded, document);
}
