use super::registry_errors;
use crate::constants::extensions;
use crate::ecs::{ScriptId, SoundId, SpriteId, TomlId};
use crate::prefab::PrefabId;
use bishop::text::FontId;
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
    Font(FontId),
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

impl From<FontId> for AssetKey {
    fn from(value: FontId) -> Self {
        Self::Font(value)
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
    Font,
}

impl AssetKind {
    /// Every supported asset kind.
    pub const ALL: [Self; 6] = [
        Self::Sprite,
        Self::Script,
        Self::Prefab,
        Self::Sound,
        Self::Toml,
        Self::Font,
    ];

    pub(crate) fn validate_extension(self, path: &Path, folder: &Path) -> io::Result<()> {
        if path.extension().is_some_and(|candidate| {
            candidate.to_str().is_some_and(|extension| {
                self.supported_extensions().contains(&extension)
            })
        }) && (!self.requires_file_stem() || path.file_stem().is_some())
        {
            return Ok(());
        }

        Err(registry_errors::wrong_extension(
            self,
            self.supported_extensions(),
            folder,
            path,
        ))
    }

    /// File extensions accepted for this asset kind, without a leading dot.
    pub const fn supported_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Sprite => &[extensions::PNG],
            Self::Script => &[extensions::LUA],
            Self::Prefab => &[extensions::PREFAB],
            Self::Sound => &[extensions::WAV],
            Self::Toml => &[extensions::TOML],
            Self::Font => &[extensions::TTF, extensions::OTF],
        }
    }

    /// Infers a unique asset kind from a supported file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        Self::ALL
            .into_iter()
            .find(|kind| kind.supported_extensions().contains(&extension))
    }

    fn requires_file_stem(self) -> bool {
        matches!(self, Self::Sound)
    }
}
