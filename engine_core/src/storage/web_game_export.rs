use crate::assets::AssetKey;
use crate::constants::{extensions, paths};
use crate::storage::resource_manifest::{ResourceEntry, ResourceKey, ResourceManifest};
use crate::storage::load_full_game_from_folder;
use crate::storage::web_game_bundle::{WebGameBundle, WEB_GAME_BUNDLE_VERSION, WEB_GAME_MANIFEST_FILE};
use crate::text::TextManager;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const RESOURCE_DIRS: [&str; 9] = [
    paths::WORLDS_FOLDER,
    paths::PAYLOADS_FOLDER,
    paths::ASSETS_FOLDER,
    paths::SCRIPTS_FOLDER,
    paths::TEXT_FOLDER,
    paths::MENUS_FOLDER,
    paths::PREFABS_FOLDER,
    paths::THEMES_FOLDER,
    paths::AUDIO_FOLDER,
];
const ROOT_FILES: [&str; 4] = [
    paths::GAME_RON,
    paths::ECS_RON,
    paths::ASSET_REGISTRY_RON,
    paths::TILE_REGISTRY_RON,
];

/// Returns the validated authored files for one Resources directory.
pub fn collect_game_resources(root: &Path) -> Result<ResourceManifest, String> {
    require_directory(root)?;
    for name in ROOT_FILES {
        require_file(&root.join(name))?;
    }

    let game = load_full_game_from_folder(root)
        .map_err(|error| format!("invalid split game data in {}: {error}", root.display()))?;
    let mut collected = BTreeMap::new();
    for name in ROOT_FILES {
        collect_file(root, &root.join(name), &mut collected)?;
    }
    let startup = root.join(paths::STARTUP_RON);
    if startup.exists() {
        collect_file(root, &startup, &mut collected)?;
    }
    for name in RESOURCE_DIRS {
        let directory = root.join(name);
        if directory.exists() {
            collect_directory(root, &directory, &mut collected)?;
        }
    }

    let text_manager = TextManager::new(root.join(paths::TEXT_FOLDER));
    let mut registered: Vec<_> = game.asset_registry.records().iter()
        .map(|(&key, record)| (key, record.path.clone()))
        .collect();
    registered.sort_by(|left, right| left.1.cmp(&right.1));
    for (asset, path) in registered {
        let resolved = match asset {
            AssetKey::Toml(id) => text_manager.text_path_for_id(&game.asset_registry, id)
                .ok_or_else(|| format!("no text path for registered asset: {}", path.display()))?,
            _ => root.join(path),
        };
        let relative = resolved.strip_prefix(root)
            .map_err(|error| format!("registered resource outside {}: {error}", root.display()))?;
        let key = ResourceKey::from_native(relative)
            .map_err(|error| format!("registered resource {}: {error}", resolved.display()))?;
        if !collected.contains_key(&key) {
            return Err(format!("registered resource missing: {}", resolved.display()));
        }
    }

    let manifest = ResourceManifest {
        files: collected.into_iter()
            .map(|(path, byte_len)| ResourceEntry { path, byte_len })
            .collect(),
    };
    manifest.validate().map_err(|error| error.to_string())?;
    Ok(manifest)
}

/// Stages authored Resources and a versioned manifest in a new directory.
pub fn stage_web_game_resources(root: &Path, destination: &Path) -> Result<(), String> {
    let resources = collect_game_resources(root)?;
    let bundle = WebGameBundle {
        format_version: WEB_GAME_BUNDLE_VERSION,
        resources,
    };
    bundle.validate().map_err(|error| error.to_string())?;

    ensure_export_destination(root, destination)?;
    fs::create_dir(destination)
        .map_err(|error| format!("failed to create new export {}: {error}", destination.display()))?;

    let result = (|| {
        for entry in &bundle.resources.files {
            let from = root.join(entry.path.as_path());
            let to = destination.join(entry.path.as_path());
            let parent = to.parent()
                .ok_or_else(|| format!("no parent for {}", to.display()))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            let copied = fs::copy(&from, &to)
                .map_err(|error| format!("failed to copy {}: {error}", from.display()))?;
            if copied != entry.byte_len as u64 {
                return Err(format!("resource length changed while copying: {}", from.display()));
            }
        }
        let manifest = ron::ser::to_string_pretty(&bundle, ron::ser::PrettyConfig::new())
            .map_err(|error| format!("failed to serialize web game manifest: {error}"))?;
        let path = destination.join(WEB_GAME_MANIFEST_FILE);
        fs::write(&path, manifest)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    })();
    if let Err(error) = result {
        fs::remove_dir_all(destination).map_err(|cleanup| {
            format!("{error}; failed to clean up {}: {cleanup}", destination.display())
        })?;
        return Err(error);
    }
    Ok(())
}

/// Checks a new export directory before any output is created.
pub(crate) fn ensure_export_destination(root: &Path, destination: &Path) -> Result<(), String> {
    let source = fs::canonicalize(root)
        .map_err(|error| format!("failed to resolve {}: {error}", root.display()))?;
    let parent = destination.parent()
        .ok_or_else(|| format!("no parent for {}", destination.display()))?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("failed to resolve {}: {error}", parent.display()))?;
    if parent.starts_with(&source) {
        return Err(format!("export destination is inside authored Resources: {}", destination.display()));
    }
    Ok(())
}

fn collect_directory(root: &Path, directory: &Path, files: &mut BTreeMap<ResourceKey, usize>) -> Result<(), String> {
    require_directory(directory)?;
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to list {}: {error}", directory.display()))?
    {
        let path = entry.map_err(|error| format!("failed to list {}: {error}", directory.display()))?.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symbolic link is not a portable resource: {}", path.display()));
        }
        if metadata.is_dir() {
            collect_directory(root, &path, files)?;
        } else if metadata.is_file() && portable_extension(&path) {
            collect_file(root, &path, files)?;
        } else if !metadata.is_file() {
            return Err(format!("not a regular resource: {}", path.display()));
        }
    }
    Ok(())
}

fn portable_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(|extension| extension.to_str()),
        Some(extensions::PNG | extensions::LUA | extensions::WAV | extensions::TOML
            | extensions::RON | extensions::PREFAB | extensions::JSON))
}

fn collect_file(root: &Path, path: &Path, files: &mut BTreeMap<ResourceKey, usize>) -> Result<(), String> {
    let metadata = file_metadata(path)?;
    let relative = path.strip_prefix(root)
        .map_err(|error| format!("resource outside {}: {error}", root.display()))?;
    let key = ResourceKey::from_native(relative)
        .map_err(|error| format!("resource {}: {error}", path.display()))?;
    let byte_len = usize::try_from(metadata.len())
        .map_err(|error| format!("resource too large {}: {error}", path.display()))?;
    if files.insert(key.clone(), byte_len).is_some() {
        return Err(format!("duplicate resource: {key}"));
    }
    Ok(())
}

fn require_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("required resource directory {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!("not a directory: {}", path.display()));
    }
    Ok(())
}

fn require_file(path: &Path) -> Result<(), String> {
    file_metadata(path).map(|_| ())
}

fn file_metadata(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("required resource {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("not a regular resource: {}", path.display()));
    }
    Ok(metadata)
}
