use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use std::path::{Component, Path, PathBuf};

/// One portable, resource-root-relative file in an authored browser export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEntry {
    /// Canonical UTF-8 path relative to the Resources root.
    pub path: String,
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
    /// Rejects unsafe, duplicate or out-of-order resource paths.
    pub fn validate(&self) -> io::Result<()> {
        let mut previous: Option<&str> = None;
        for entry in &self.files {
            checked_manifest_path(&entry.path)?;
            if let Some(previous) = previous {
                if previous == entry.path {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("duplicate resource path: {}", entry.path),
                    ));
                }
                if previous > entry.path.as_str() {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("resource paths must be sorted: {} before {previous}", entry.path),
                    ));
                }
            }
            previous = Some(&entry.path);
        }
        Ok(())
    }
}

pub(crate) fn checked_manifest_path(value: &str) -> io::Result<PathBuf> {
    let path = Path::new(value);
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(invalid_path(value));
        };
        let segment = segment.to_str().ok_or_else(|| invalid_path(value))?;
        if segment.contains(['\\', ':']) {
            return Err(invalid_path(value));
        }
        parts.push(segment);
    }
    if parts.is_empty() || parts.join("/") != value {
        return Err(invalid_path(value));
    }
    Ok(path.to_path_buf())
}

fn invalid_path(value: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, format!("unsafe resource path: {value}"))
}
