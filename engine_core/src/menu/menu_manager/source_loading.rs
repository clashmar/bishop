use super::MenuManager;
use crate::constants::{extensions, paths};
use crate::menu::MenuTemplate;
use crate::storage::ResourceSource;
use std::path::Path;

impl MenuManager {
    /// Registers authored menu templates from prefetched game resources.
    pub fn load_templates_from_source(&mut self, source: &ResourceSource) -> Result<(), String> {
        let directory = Path::new(paths::MENUS_FOLDER);
        for path in source.prefetched_files_under(directory)
            .map_err(|error| format!("list authored menus: {error}"))?
        {
            if path.as_path().parent() != Some(directory)
                || path.as_path().extension().is_none_or(|extension| extension != extensions::RON)
            {
                continue;
            }
            let bytes = source.read_required(path.as_path())
                .map_err(|error| format!("read authored menu {path}: {error}"))?;
            let contents = std::str::from_utf8(&bytes)
                .map_err(|error| format!("decode authored menu {path}: {error}"))?;
            let template: MenuTemplate = ron::from_str(contents)
                .map_err(|error| format!("parse authored menu {path}: {error}"))?;
            self.register_template(template);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::MenuManager;
    use crate::constants::paths;
    use crate::menu::MenuTemplate;
    use crate::storage::{ResourceEntry, ResourceKey, ResourceManifest, ResourceSource};
    use std::collections::HashMap;
    use std::path::Path;

    fn source_with_menu(path: &Path, bytes: Vec<u8>) -> Result<ResourceSource, String> {
        let key = ResourceKey::from_native(path).map_err(|error| error.to_string())?;
        let manifest = ResourceManifest {
            files: vec![ResourceEntry { path: key.clone(), byte_len: bytes.len() }],
        };
        ResourceSource::from_prefetched(manifest, HashMap::from([(key, bytes)]))
            .map_err(|error| error.to_string())
    }

    #[test]
    fn menu_templates_when_prefetched_register_authored_title() -> Result<(), String> {
        let path = Path::new(paths::MENUS_FOLDER).join("title.ron");
        let title = MenuTemplate::new("authored_title".into());
        let bytes = ron::to_string(&title).map_err(|error| error.to_string())?.into_bytes();
        let source = source_with_menu(&path, bytes)?;
        let mut menus = MenuManager::new();

        menus.load_templates_from_source(&source)?;

        assert!(menus.get_template(&title.id).is_some());
        Ok(())
    }

    #[test]
    fn menu_templates_when_prefetched_ron_is_invalid_reports_path() -> Result<(), String> {
        let path = Path::new(paths::MENUS_FOLDER).join("broken.ron");
        let source = source_with_menu(&path, b"not a menu".to_vec())?;
        let mut menus = MenuManager::new();

        let error = menus.load_templates_from_source(&source).err()
            .ok_or_else(|| "malformed authored menu unexpectedly loaded".to_owned())?;

        let key = ResourceKey::from_native(&path).map_err(|error| error.to_string())?;
        assert!(error.contains(key.as_str()), "{error}");
        Ok(())
    }
}
