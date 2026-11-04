use crate::constants::paths;
use crate::storage::resource_manifest::ResourceManifest;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};

/// Current schema version of an authored web game bundle.
pub const WEB_GAME_BUNDLE_VERSION: u32 = 1;
/// Filename for the versioned web game manifest.
pub const WEB_GAME_MANIFEST_FILE: &str = "manifest.ron";
/// Maximum number of eager-prefetched authored files in one player bundle.
pub const MAX_WEB_GAME_FILES: usize = 1024;
/// Maximum number of declared authored resource bytes in one player bundle.
pub const MAX_WEB_GAME_BYTES: usize = 32 * 1024 * 1024;

/// Describes authored Resources compatible with this browser player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebGameBundle {
    /// Schema version expected by the player.
    pub format_version: u32,
    /// Sorted, validated authored resource files.
    pub resources: ResourceManifest,
}

impl WebGameBundle {
    /// Validates the bundle schema and its required authored game data.
    pub fn validate(&self) -> io::Result<()> {
        if self.format_version != WEB_GAME_BUNDLE_VERSION {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("unsupported web game bundle version: {}", self.format_version),
            ));
        }
        self.resources.validate()?;
        if !self.resources.files.iter().any(|entry| entry.path.as_str() == paths::GAME_RON) {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("required resource missing: {}", paths::GAME_RON),
            ));
        }
        if self.resources.files.len() > MAX_WEB_GAME_FILES {
            return Err(io::Error::new(ErrorKind::InvalidData,
                format!("web game bundle exceeds {MAX_WEB_GAME_FILES} declared files")));
        }
        let bytes = self.resources.files.iter().try_fold(0usize, |sum, entry| {
            sum.checked_add(entry.byte_len)
        }).ok_or_else(|| io::Error::new(ErrorKind::InvalidData,
            "web game bundle declared byte count overflows"))?;
        if bytes > MAX_WEB_GAME_BYTES {
            return Err(io::Error::new(ErrorKind::InvalidData,
                "web game bundle exceeds 32 MiB of declared game bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{WebGameBundle, WEB_GAME_BUNDLE_VERSION};
    use crate::constants::paths;
    use crate::storage::{ResourceEntry, ResourceKey, ResourceManifest};

    fn bundle(files: Vec<ResourceEntry>) -> WebGameBundle {
        WebGameBundle {
            format_version: WEB_GAME_BUNDLE_VERSION,
            resources: ResourceManifest { files },
        }
    }

    fn entry(path: &str) -> ResourceEntry {
        ResourceEntry { path: ResourceKey::from_wire(path).unwrap(), byte_len: 1 }
    }

    #[test]
    fn bundle_when_version_and_game_are_valid_round_trips() {
        let source = bundle(vec![entry(paths::GAME_RON)]);
        source.validate().unwrap();
        let serialized = ron::to_string(&source).unwrap();
        let decoded: WebGameBundle = ron::from_str(&serialized).unwrap();
        assert_eq!(decoded, source);
    }

    #[test]
    fn bundle_when_version_is_unsupported_names_the_version() {
        let mut source = bundle(vec![entry(paths::GAME_RON)]);
        source.format_version += 1;
        let error = source.validate().unwrap_err();
        assert!(error.to_string().contains(&source.format_version.to_string()));
    }

    #[test]
    fn bundle_when_game_file_is_missing_names_it() {
        let error = bundle(vec![]).validate().unwrap_err();
        assert!(error.to_string().contains(paths::GAME_RON));
    }

    #[test]
    fn bundle_when_path_is_unsafe_uses_manifest_validation() {
        let serialized = format!(
            r#"(format_version:{WEB_GAME_BUNDLE_VERSION},resources:(files:[(path:"../unsafe",byte_len:1)]))"#,
        );
        let error = ron::from_str::<WebGameBundle>(&serialized).unwrap_err();
        assert!(error.to_string().contains("../unsafe"));
    }

    #[test]
    fn bundle_when_more_than_1024_files_rejects_export() {
        let mut files = (0..1024).map(|index| entry(&format!("assets/file{index:04}.bin")))
            .collect::<Vec<_>>();
        files.push(entry(paths::GAME_RON));
        let error = bundle(files).validate().unwrap_err();
        assert!(error.to_string().contains("1024"), "{error}");
    }

    #[test]
    fn bundle_when_more_than_32_mib_of_resources_rejects_export() {
        let mut game = entry(paths::GAME_RON);
        game.byte_len = 32 * 1024 * 1024 + 1;
        let error = bundle(vec![game]).validate().unwrap_err();
        assert!(error.to_string().contains("32 MiB"), "{error}");
    }

    #[test]
    fn bundle_when_declared_size_overflows_rejects_export() {
        let mut first = entry("assets/first.bin");
        first.byte_len = usize::MAX;
        let error = bundle(vec![first, entry(paths::GAME_RON)]).validate().unwrap_err();
        assert!(error.to_string().contains("overflows"), "{error}");
    }
}
