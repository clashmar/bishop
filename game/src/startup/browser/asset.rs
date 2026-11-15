use crate::engine::EngineEntryMode;
use crate::startup::StartupAsset;
use engine_core::constants::paths;
use engine_core::storage::ResourceSource;
use std::path::Path;

/// Selects the authored browser entry flow from prefetched startup data.
pub(super) fn browser_entry_mode(source: &ResourceSource) -> Result<EngineEntryMode, String> {
    let path = Path::new(paths::STARTUP_RON);
    let Some(bytes) = source.read_optional(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?
    else {
        return Ok(EngineEntryMode::Playing);
    };
    let contents = std::str::from_utf8(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))?;
    let startup: StartupAsset = ron::from_str(contents)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    let menu_id = startup.title_menu_id.trim();
    if menu_id.is_empty() {
        return Ok(EngineEntryMode::Playing);
    }
    Ok(EngineEntryMode::StartMenu { menu_id: menu_id.to_owned() })
}
