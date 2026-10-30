use crate::constants::paths;
use crate::storage::resource_manifest::ResourceManifest;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};

/// Current schema version of an authored web game bundle.
pub const WEB_GAME_BUNDLE_VERSION: u32 = 1;
/// Filename for the versioned web game manifest.
pub const WEB_GAME_MANIFEST_FILE: &str = "manifest.ron";

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
        if !self.resources.files.iter().any(|entry| entry.path == paths::GAME_RON) {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("required resource missing: {}", paths::GAME_RON),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{WebGameBundle, WEB_GAME_BUNDLE_VERSION};
    use crate::constants::paths;
    use crate::storage::{ResourceEntry, ResourceManifest};

    fn bundle(files: Vec<ResourceEntry>) -> WebGameBundle {
        WebGameBundle {
            format_version: WEB_GAME_BUNDLE_VERSION,
            resources: ResourceManifest { files },
        }
    }

    fn entry(path: &str) -> ResourceEntry {
        ResourceEntry { path: path.to_owned(), byte_len: 1 }
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
        let error = bundle(vec![entry("../unsafe"), entry(paths::GAME_RON)])
            .validate().unwrap_err();
        assert!(error.to_string().contains("../unsafe"));
    }
}
