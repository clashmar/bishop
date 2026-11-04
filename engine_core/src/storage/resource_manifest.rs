use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Component, Path};

/// A validated, portable path relative to the Resources root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Converts a native relative path to a portable resource key.
    pub fn from_native(path: &Path) -> io::Result<Self> {
        let mut parts = Vec::new();
        for component in path.components() {
            let Component::Normal(segment) = component else {
                return Err(io::Error::new(ErrorKind::InvalidInput, "unsafe resource path"));
            };
            let segment = segment.to_str().ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidInput, "resource path is not valid UTF-8")
            })?;
            if !valid_segment(segment) {
                return Err(io::Error::new(ErrorKind::InvalidInput, "unsafe resource path"));
            }
            parts.push(segment);
        }
        if parts.is_empty() {
            return Err(io::Error::new(ErrorKind::InvalidInput, "empty resource path"));
        }
        Ok(Self(parts.join("/")))
    }

    /// Parses a canonical resource path from a manifest.
    pub fn from_wire(value: &str) -> io::Result<Self> {
        if value.split('/').any(|segment| !valid_segment(segment)) {
            return Err(invalid_path(value));
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical slash-separated path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a relative path usable for native filesystem lookups.
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for ResourceKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ResourceKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::from_wire(&value).map_err(serde::de::Error::custom)
    }
}

/// One portable, resource-root-relative file in an authored browser export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEntry {
    /// Canonical UTF-8 path relative to the Resources root.
    pub path: ResourceKey,
    /// Expected number of bytes in the staged file.
    pub byte_len: usize,
}

/// Ordered set of authored resource files required by a browser export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceManifest {
    /// Resource entries in ascending path order.
    pub files: Vec<ResourceEntry>,
}

impl ResourceManifest {
    /// Rejects duplicate or out-of-order resource paths.
    pub fn validate(&self) -> io::Result<()> {
        let mut previous: Option<&ResourceKey> = None;
        let mut casefolded = HashSet::new();
        for entry in &self.files {
            if !casefolded.insert(entry.path.as_str().to_lowercase()) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("duplicate resource path on case-insensitive filesystems: {}", entry.path),
                ));
            }
            if let Some(previous) = previous
                && previous > &entry.path
            {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("resource paths must be sorted: {} before {previous}", entry.path),
                ));
            }
            previous = Some(&entry.path);
        }
        Ok(())
    }
}

fn valid_segment(segment: &str) -> bool {
    if segment.is_empty() || matches!(segment, "." | "..")
        || segment.ends_with(['.', ' '])
        || segment.chars().any(|c| c.is_control() || matches!(c, '\\' | ':' | '?' | '*' | '"' | '<' | '>' | '|'))
    {
        return false;
    }
    let stem = segment.split('.').next().unwrap_or_default();
    !matches!(stem.to_ascii_uppercase().as_str(), "CON" | "PRN" | "AUX" | "NUL"
        | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
        | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9")
}

fn invalid_path(value: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, format!("unsafe resource path: {value}"))
}

#[cfg(test)]
mod tests {
    use super::{ResourceEntry, ResourceKey, ResourceManifest};
    use std::path::Path;

    #[test]
    fn manifest_when_deserializing_unsafe_path_rejects_it() {
        let serialized = r#"(files:[(path:"../unsafe",byte_len:1)])"#;
        assert!(ron::from_str::<ResourceManifest>(serialized).is_err());
    }

    #[test]
    fn resource_key_when_native_path_has_multiple_components_serializes_with_slashes() {
        let native = Path::new("menus").join("title.ron");
        let key = ResourceKey::from_native(&native).unwrap();
        let encoded = ron::to_string(&key).unwrap();

        assert_eq!(key.as_str(), "menus/title.ron");
        assert_eq!(encoded, "\"menus/title.ron\"");
        assert_eq!(ron::from_str::<ResourceKey>(&encoded).unwrap(), key);
    }

    #[test]
    fn resource_key_when_windows_forbidden_name_is_received_rejects_on_every_host() {
        for value in ["assets/con.png", "assets/COM1.txt", "assets/trailing. ",
            "assets/invalid?.png", "assets/invalid*.png", "assets/invalid|.png",
            "assets/invalid\".png", "assets/invalid<.png", "assets/invalid>.png"]
        {
            assert!(ResourceKey::from_wire(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn manifest_when_paths_alias_on_windows_rejects_them() {
        let files = ["assets/Hero.png", "assets/hero.png"].map(|path| ResourceEntry {
            path: ResourceKey::from_wire(path).unwrap(), byte_len: 1,
        });
        let manifest = ResourceManifest { files: files.into() };

        assert!(manifest.validate().unwrap_err().to_string().contains("duplicate"));
    }
}
