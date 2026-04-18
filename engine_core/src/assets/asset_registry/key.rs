use crate::ecs::{ScriptId, SpriteId};
use crate::prefab::PrefabId;
use serde::{Deserialize, Serialize};

/// Stable cross-system asset key for asset kinds that already have typed ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AssetKey {
    Sprite(SpriteId),
    Script(ScriptId),
    Prefab(PrefabId),
}

/// Asset kind metadata stored alongside the canonical project-relative path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Sprite,
    Script,
    Prefab,
}
