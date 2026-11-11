use engine_core::storage::web_game_export::collect_game_resources;
use std::fs;
use std::path::Path;

/// Copies the validated Demo files and their manifest into a browser stage.
pub(super) fn stage_demo_resources(root: &Path, stage: &Path) -> Result<(), String> {
    let manifest = collect_game_resources(root)?;
    fs::create_dir_all(stage).map_err(|error| format!("failed to create {}: {error}", stage.display()))?;
    for entry in &manifest.files {
        let source = root.join(entry.path.as_path());
        let destination = stage.join(entry.path.as_path());
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
