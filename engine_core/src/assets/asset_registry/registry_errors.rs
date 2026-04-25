use super::{AssetKey, AssetKind};
use std::io::{Error, ErrorKind};
use std::path::Path;

pub(super) fn invalid_record(key: AssetKey, error: Error) -> Error {
    Error::new(
        error.kind(),
        format!("Invalid asset registry record '{key:?}': {error}"),
    )
}

pub(super) fn conflicting_path(path: &Path, existing_key: AssetKey, key: AssetKey) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!(
            "Asset path '{}' maps to both '{existing_key:?}' and '{key:?}'",
            path.display()
        ),
    )
}

pub(super) fn conflicting_registry_path(
    path: &Path,
    existing_key: AssetKey,
    key: AssetKey,
) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!(
            "Asset registry path '{}' maps to both '{existing_key:?}' and '{key:?}'",
            path.display()
        ),
    )
}

pub(super) fn key_maps_to_multiple_records(key: AssetKey) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("Asset key '{key:?}' maps to multiple records"),
    )
}

pub(super) fn rooted_path(kind: AssetKind, folder: &Path, path: &Path) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!(
            "{kind:?} paths must live under '{}': '{}'",
            folder.display(),
            path.display()
        ),
    )
}

pub(super) fn canonical_spelling(kind: AssetKind, folder: &Path, path: &Path) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!(
            "{kind:?} paths must use canonical spelling under '{}': '{}'",
            folder.display(),
            path.display()
        ),
    )
}

pub(super) fn clean_relative_path(kind: AssetKind, folder: &Path, path: &Path) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!(
            "{kind:?} paths must be clean relative paths under '{}': '{}'",
            folder.display(),
            path.display()
        ),
    )
}

pub(super) fn wrong_extension(
    kind: AssetKind,
    extension: &str,
    folder: &Path,
    path: &Path,
) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!(
            "{kind:?} paths must point to '.{extension}' files under '{}': '{}'",
            folder.display(),
            path.display()
        ),
    )
}
