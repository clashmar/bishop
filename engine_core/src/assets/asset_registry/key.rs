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

impl From<SpriteId> for AssetKey {
    fn from(value: SpriteId) -> Self {
        Self::Sprite(value)
    }
}

impl From<ScriptId> for AssetKey {
    fn from(value: ScriptId) -> Self {
        Self::Script(value)
    }
}

impl From<PrefabId> for AssetKey {
    fn from(value: PrefabId) -> Self {
        Self::Prefab(value)
    }
}

/// Asset kind metadata stored alongside the canonical project-relative path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Sprite,
    Script,
    Prefab,
}
