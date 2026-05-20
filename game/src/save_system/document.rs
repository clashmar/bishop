use super::{SaveLane, SaveSlotKey};
use engine_core::storage::ordered_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error};
use std::path::Path;
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

impl RuntimeSaveDocument {
    pub fn to_ron_string(&self) -> io::Result<String> {
        let pretty = ron::ser::PrettyConfig::new()
            .separate_tuple_members(false)
            .enumerate_arrays(true);
        ron::ser::to_string_pretty(self, pretty).map_err(Error::other)
    }

    pub fn from_ron_str(ron: &str) -> io::Result<Self> {
        ron::from_str(ron).map_err(Error::other)
    }

    pub fn write_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_ron_string()?)
    }

    pub fn read_from_path(path: &Path) -> io::Result<Self> {
        let ron = fs::read_to_string(path)?;
        Self::from_ron_str(&ron)
    }
}
