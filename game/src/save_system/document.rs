use super::{RonPersist, SaveLane, SaveSlotKey};
use engine_core::storage::ordered_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSection {
    pub version: u32,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSaveMetadata {
    pub schema_version: u32,
    pub game_id: Uuid,
    pub game_name: String,
    pub lane: SaveLane,
    pub slot: SaveSlotKey,
    pub saved_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSaveDocument {
    pub metadata: RuntimeSaveMetadata,
    #[serde(
        serialize_with = "ordered_map::serialize",
        deserialize_with = "ordered_map::deserialize"
    )]
    pub sections: HashMap<String, SavedSection>,
}

impl RonPersist for RuntimeSaveDocument {
    fn ron_config() -> ron::ser::PrettyConfig {
        ron::ser::PrettyConfig::new()
            .separate_tuple_members(false)
            .enumerate_arrays(true)
    }
}
