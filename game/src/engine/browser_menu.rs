use crate::engine::EngineEntryMode;
use engine_core::menu::MenuManager;
use engine_core::storage::ResourceSource;

/// Creates the menu manager for an authored browser game.
pub(super) fn menu_manager_from_source(
    source: &ResourceSource,
    entry_mode: &EngineEntryMode,
) -> Result<MenuManager, String> {
    let mut menus = MenuManager::new();
    menus.load_templates_from_source(source)?;
    if let EngineEntryMode::StartMenu { menu_id } = entry_mode {
        if menus.get_template(menu_id).is_none() {
            return Err(format!("authored title menu '{menu_id}' is missing"));
        }
    }
    Ok(menus)
}

#[cfg(test)]
mod tests {
    use super::menu_manager_from_source;
    use crate::engine::EngineEntryMode;
    use engine_core::constants::paths;
    use engine_core::menu::MenuTemplate;
    use engine_core::storage::{ResourceEntry, ResourceKey, ResourceManifest, ResourceSource};
    use std::collections::HashMap;
    use std::path::Path;

    fn menu_source(path: &Path, bytes: Vec<u8>) -> ResourceSource {
        let key = ResourceKey::from_native(path).unwrap();
        let files = vec![ResourceEntry { path: key.clone(), byte_len: bytes.len() }];
        ResourceSource::from_prefetched(
            ResourceManifest { files }, HashMap::from([(key, bytes)]),
        ).unwrap()
    }

    #[test]
    fn browser_menu_when_title_is_prefetched_registers_it_for_engine_start() {
        let menu = MenuTemplate::new("authored_title".into());
        let path = Path::new(paths::MENUS_FOLDER).join("title.ron");
        let bytes = ron::to_string(&menu).unwrap().into_bytes();
        let source = menu_source(&path, bytes);
        let entry_mode = EngineEntryMode::StartMenu { menu_id: menu.id.clone() };

        let manager = menu_manager_from_source(&source, &entry_mode).unwrap();

        assert!(manager.get_template(&menu.id).is_some());
    }

    #[test]
    fn browser_menu_when_title_is_missing_reports_authored_id() {
        let source = ResourceSource::from_prefetched(
            ResourceManifest { files: vec![] }, HashMap::new(),
        ).unwrap();
        let entry_mode = EngineEntryMode::StartMenu { menu_id: "missing_title".into() };

        let error = menu_manager_from_source(&source, &entry_mode).err().unwrap();

        assert!(error.contains("missing_title"), "{error}");
    }
}
