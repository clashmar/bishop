mod split_layout;
#[cfg(test)]
mod split_layout_tests;
#[cfg(test)]
mod tests;

pub use split_layout::{
    hydrate_current_payloads_from_source, hydrate_room_from_source, load_game_shell_from_source,
};
pub(crate) use split_layout::{
    hydrate_all_payloads_from_source, validate_global_room_membership,
};

use crate::constants::{extensions, paths};
use crate::storage::resource_manifest::{ResourceManifest, checked_manifest_path};
use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Reads authored game data from a native folder or prefetched browser bytes.
#[derive(Clone, Debug)]
pub enum ResourceSource {
    /// Reads resources beneath a native directory.
    Folder(PathBuf),
    /// Reads previously fetched bytes without filesystem access.
    Prefetched(Arc<HashMap<PathBuf, Vec<u8>>>),
}

impl ResourceSource {
    /// Builds a byte source after validating every declared file and its length.
    pub fn from_prefetched(
        manifest: ResourceManifest,
        mut files: HashMap<String, Vec<u8>>,
    ) -> io::Result<Self> {
        manifest.validate()?;
        let mut indexed = HashMap::with_capacity(manifest.files.len());
        for entry in manifest.files {
            let relative = checked_manifest_path(&entry.path)?;
            let bytes = files.remove(&entry.path).ok_or_else(|| {
                io::Error::new(ErrorKind::NotFound, format!("required resource missing: {}", entry.path))
            })?;
            if bytes.len() != entry.byte_len {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("resource length mismatch for {}: expected {}, got {}", entry.path, entry.byte_len, bytes.len()),
                ));
            }
            indexed.insert(relative, bytes);
        }
        if let Some(extra) = files.keys().min() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("undeclared fetched resource: {extra}"),
            ));
        }
        Ok(Self::Prefetched(Arc::new(indexed)))
    }

    /// Lists prefetched files below a relative resource directory in path order.
    pub fn prefetched_files_under(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        checked_lookup(directory)?;
        let Self::Prefetched(files) = self else {
            return Err(io::Error::new(ErrorKind::Unsupported, "resource files are not prefetched"));
        };
        let mut paths: Vec<_> = files.keys()
            .filter(|path| path.starts_with(directory) && path.as_path() != directory)
            .cloned()
            .collect();
        paths.sort();
        Ok(paths)
    }

    /// Returns prefetched Luau modules under scripts/ and themes/, rejecting invalid UTF-8.
    pub fn prefetched_lua_sources(&self) -> io::Result<HashMap<String, String>> {
        let Self::Prefetched(files) = self else {
            return Err(io::Error::new(ErrorKind::Unsupported, "Lua modules are not prefetched"));
        };
        let mut modules: Vec<_> = files.iter().filter(|(path, _)| {
            (path.starts_with(paths::SCRIPTS_FOLDER) || path.starts_with(paths::THEMES_FOLDER))
                && path.extension().is_some_and(|extension| extension == extensions::LUA)
        }).collect();
        modules.sort_by_key(|(path, _)| *path);

        let mut sources = HashMap::with_capacity(modules.len());
        for (path, bytes) in modules {
            let key = path.components().map(|component| component.as_os_str().to_str()
                .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, format!("non-UTF8 Lua path: {}", path.display()))))
                .collect::<io::Result<Vec<_>>>()?.join("/");
            let source = std::str::from_utf8(bytes).map_err(|error| {
                io::Error::new(ErrorKind::InvalidData, format!("invalid UTF-8 in {key}: {error}"))
            })?;
            sources.insert(key, source.to_owned());
        }
        Ok(sources)
    }

    /// Returns the bytes at a relative resource path or an error naming it.
    pub fn read_required(&self, relative: &Path) -> io::Result<Vec<u8>> {
        self.read_optional(relative)?.ok_or_else(|| {
            let path = match self {
                Self::Folder(folder) => folder.join(relative),
                Self::Prefetched(_) => relative.to_path_buf(),
            };
            io::Error::new(
                ErrorKind::NotFound,
                format!("required resource missing: {}", path.display()),
            )
        })
    }

    /// Returns bytes at a relative resource path, or `None` if absent.
    pub fn read_optional(&self, relative: &Path) -> io::Result<Option<Vec<u8>>> {
        checked_lookup(relative)?;
        match self {
            Self::Folder(folder) => match fs::read(folder.join(relative)) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(io::Error::new(
                    error.kind(),
                    format!("failed to read {}: {error}", folder.join(relative).display()),
                )),
            },
            Self::Prefetched(files) => Ok(files.get(relative).cloned()),
        }
    }
}
fn checked_lookup(relative: &Path) -> io::Result<()> {
    if relative.components().next().is_none()
        || !relative.components().all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("unsafe resource lookup: {}", relative.display()),
        ));
    }
    Ok(())
}
