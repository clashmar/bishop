use super::{SaveLane, SaveSlotKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Error};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestRuntimeSaveManifest {
    pub lane: SaveLane,
    pub slot: SaveSlotKey,
    pub saved_at_unix_ms: u64,
}

impl LatestRuntimeSaveManifest {
    pub fn to_ron_string(&self) -> io::Result<String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new()).map_err(Error::other)
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
