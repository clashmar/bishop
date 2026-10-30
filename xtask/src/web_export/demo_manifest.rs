use engine_core::assets::AssetKey;
use engine_core::constants::{extensions, paths};
use engine_core::storage::{
    ResourceEntry, ResourceManifest, load_full_game_from_folder,
};
use engine_core::text::TextManager;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

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
pub(super) fn collect_demo_resources(root: &Path) -> Result<ResourceManifest, String> {
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
        let key = relative_key(relative)?;
        if !collected.contains_key(&key) {
            return Err(format!("registered Demo resource missing: {}", resolved.display()));
        }
    }

    let manifest = ResourceManifest {
        files: collected.into_iter().map(|(path, byte_len)| ResourceEntry { path, byte_len }).collect(),
    };
    manifest.validate().map_err(|error| error.to_string())?;
    Ok(manifest)
}

/// Copies the validated Demo files and their manifest into a browser stage.
pub(super) fn stage_demo_resources(root: &Path, stage: &Path) -> Result<(), String> {
    let manifest = collect_demo_resources(root)?;
    fs::create_dir_all(stage).map_err(|error| format!("failed to create {}: {error}", stage.display()))?;
    for entry in &manifest.files {
        let source = root.join(&entry.path);
        let destination = stage.join(&entry.path);
        let parent = destination.parent().ok_or_else(|| format!("no parent for {}", destination.display()))?;
        fs::create_dir_all(parent).map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        let bytes = fs::copy(&source, &destination)
            .map_err(|error| format!("failed to stage {}: {error}", source.display()))?;
        if bytes != entry.byte_len as u64 {
            return Err(format!("staged resource length changed: {}", source.display()));
        }
    }
    let manifest_path = stage.join(web_shell::DEMO_CHECKPOINT_MANIFEST_FILE);
    let contents = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::new())
        .map_err(|error| format!("failed to serialize Demo manifest: {error}"))?;
    fs::write(&manifest_path, contents)
        .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))
}

fn collect_directory(root: &Path, directory: &Path, files: &mut BTreeMap<String, usize>) -> Result<(), String> {
    require_directory(directory)?;
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to list {}: {error}", directory.display()))?
    {
        let path = entry.map_err(|error| format!("failed to list {}: {error}", directory.display()))?.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symbolic link is not a portable Demo resource: {}", path.display()));
        }
        if metadata.is_dir() {
            collect_directory(root, &path, files)?;
        } else if metadata.is_file() && portable_extension(&path) {
            collect_file(root, &path, files)?;
        } else if !metadata.is_file() {
            return Err(format!("not a regular Demo resource: {}", path.display()));
        }
    }
    Ok(())
}

fn portable_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(|extension| extension.to_str()),
        Some(extensions::PNG | extensions::LUA | extensions::WAV | extensions::TOML
            | extensions::RON | extensions::PREFAB | extensions::JSON))
}

fn collect_file(root: &Path, path: &Path, files: &mut BTreeMap<String, usize>) -> Result<(), String> {
    let metadata = file_metadata(path)?;
    let relative = path.strip_prefix(root)
        .map_err(|error| format!("resource outside {}: {error}", root.display()))?;
    let key = relative_key(relative)?;
    let byte_len = usize::try_from(metadata.len())
        .map_err(|error| format!("resource too large {}: {error}", path.display()))?;
    if files.insert(key.clone(), byte_len).is_some() {
        return Err(format!("duplicate Demo resource: {key}"));
    }
    Ok(())
}

fn relative_key(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(format!("unsafe Demo resource path: {}", path.display()));
        };
        parts.push(part.to_str().ok_or_else(|| format!("non-UTF8 Demo resource: {}", path.display()))?);
    }
    if parts.is_empty() {
        return Err(format!("empty Demo resource path: {}", path.display()));
    }
    Ok(parts.join("/"))
}

fn require_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("required Demo resource directory {}: {error}", path.display()))?;
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
        .map_err(|error| format!("required Demo resource {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("not a regular Demo resource: {}", path.display()));
    }
    Ok(metadata)
}
