use super::registry_errors;
use crate::ecs::{ScriptId, SoundId, SpriteId, TomlId};
use crate::prefab::PrefabId;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Stable cross-system asset key for asset kinds that already have typed ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AssetKey {
    Sprite(SpriteId),
    Script(ScriptId),
    Prefab(PrefabId),
    Sound(SoundId),
    Toml(TomlId),
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

impl From<SoundId> for AssetKey {
    fn from(value: SoundId) -> Self {
        Self::Sound(value)
    }
}

impl From<TomlId> for AssetKey {
    fn from(value: TomlId) -> Self {
        Self::Toml(value)
    }
}

/// Asset kind metadata stored alongside the canonical project-relative path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Sprite,
    Script,
    Prefab,
    Sound,
    Toml,
}

impl AssetKind {
    pub(crate) fn validate_extension(self, path: &Path, folder: &Path) -> io::Result<()> {
        let Some(extension) = self.required_extension() else {
            return Ok(());
        };

        if path
            .extension()
            .is_some_and(|candidate| candidate == extension)
            && (!self.requires_file_stem() || path.file_stem().is_some())
        {
            return Ok(());
        }

        Err(registry_errors::wrong_extension(
            self, extension, folder, path,
        ))
    }

    fn required_extension(self) -> Option<&'static str> {
        match self {
            Self::Sound => Some("wav"),
            Self::Toml => Some("toml"),
            _ => None,
        }
    }

    fn requires_file_stem(self) -> bool {
        matches!(self, Self::Sound)
    }
}
