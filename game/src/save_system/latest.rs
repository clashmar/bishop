use super::{RonPersist, SaveLane, SaveSlotKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestRuntimeSaveManifest {
    pub lane: SaveLane,
    pub slot: SaveSlotKey,
    pub game_name: String,
    pub saved_at_unix_ms: u64,
}

impl RonPersist for LatestRuntimeSaveManifest {}
