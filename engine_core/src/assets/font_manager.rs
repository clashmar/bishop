use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use bishop::text::{FontId, Text};

use crate::assets::asset_registry::{AssetKey, AssetRegistry};
use crate::constants::paths;
use crate::logging::omni_error;
use crate::storage::{ResourceSource, fonts_folder};
use crate::text::GameTextSettings;

#[derive(Default)]
pub struct FontManager {
    registrations: HashMap<FontId, FontRegistration>,
    missing: HashSet<FontId>,
}

struct FontRegistration {
    path: PathBuf,
    available: bool,
}

impl FontManager {
    /// Loads an element override or game-default font when it is registered.
    pub fn ensure_element_font<T: Text>(
        &mut self,
        ctx: &mut T,
        registry: &AssetRegistry,
        source: Option<&ResourceSource>,
        settings: &GameTextSettings,
        override_font: Option<FontId>,
    ) -> Option<FontId> {
        let font_id = settings.font_for_element(override_font)?;
        self.ensure_font(ctx, registry, source, font_id)
    }

    /// Loads a registered font into the renderer and returns its selection when available.
    pub fn ensure_font<T: Text>(
        &mut self,
        ctx: &mut T,
        registry: &AssetRegistry,
        source: Option<&ResourceSource>,
        id: FontId,
    ) -> Option<FontId> {
        let key = AssetKey::Font(id);
        if registry.record(key).is_none() {
            if self.missing.insert(id) {
                omni_error!("Menu label references unregistered font {}", id.0);
            }
            self.registrations.remove(&id);
            return None;
        }
        self.missing.remove(&id);
        let relative_path = registry.relative_path(id)?;
        if let Some(registration) = self.registrations.get(&id)
            && registration.path == relative_path
        {
            return registration.available.then_some(id);
        }

        let resource_path = Path::new(paths::FONTS_FOLDER).join(&relative_path);
        let bytes = source.map_or_else(
            || fs::read(fonts_folder().join(&relative_path)).map_err(|error| error.to_string()),
            |source| source.read_required(&resource_path).map_err(|error| error.to_string()),
        );
        let result = bytes.and_then(|bytes| ctx.register_font(id, &bytes));
        let available = result.is_ok();
        if let Err(error) = result {
            omni_error!("Failed to load font '{}': {error}", resource_path.display());
        }
        self.registrations.insert(
            id,
            FontRegistration {
                path: relative_path,
                available,
            },
        );
        available.then_some(id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use bishop::text::{FontId, Text, TextDimensions, TextParams};
    use bishop::types::Color;

    use crate::assets::asset_registry::AssetRegistry;
    use crate::constants::{extensions, paths};
    use crate::storage::{ResourceEntry, ResourceKey, ResourceManifest, ResourceSource};

    use super::FontManager;

    struct FontTestContext {
        registered: HashMap<FontId, usize>,
        reject_registration: bool,
    }

    impl Text for FontTestContext {
        fn draw_text(
            &mut self,
            _text: &str,
            _x: f32,
            _y: f32,
            _font_size: f32,
            _color: Color,
        ) -> TextDimensions {
            TextDimensions::default()
        }

        fn draw_text_ex(
            &mut self,
            _text: &str,
            _x: f32,
            _y: f32,
            _params: TextParams,
        ) -> TextDimensions {
            TextDimensions::default()
        }

        fn measure_text(&self, _text: &str, _font_size: f32) -> TextDimensions {
            TextDimensions::default()
        }

        fn register_font(&mut self, id: FontId, data: &[u8]) -> Result<(), String> {
            if self.reject_registration {
                return Err("invalid font bytes".to_owned());
            }
            self.registered.insert(id, data.len());
            Ok(())
        }
    }

    fn font_fixture(
        id: FontId,
        bytes: Vec<u8>,
    ) -> Result<(AssetRegistry, ResourceSource), String> {
        let file_name = format!("body.{}", extensions::TTF);
        let relative_path = PathBuf::from(&file_name);
        let resource_path = PathBuf::from(paths::FONTS_FOLDER).join(&relative_path);
        let key = ResourceKey::from_native(&resource_path).map_err(|error| error.to_string())?;
        let manifest = ResourceManifest {
            files: vec![ResourceEntry {
                path: key.clone(),
                byte_len: bytes.len(),
            }],
        };
        let source = ResourceSource::from_prefetched(
            manifest,
            HashMap::from([(key, bytes)]),
        )
        .map_err(|error| error.to_string())?;
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(id, &relative_path)
            .map_err(|error| error.to_string())?;
        Ok((registry, source))
    }

    #[test]
    fn ensure_font_registers_prefetched_bytes_once() -> Result<(), String> {
        let id = FontId(42);
        let bytes = include_bytes!("fonts/gnf.regular.ttf").to_vec();
        let (registry, source) = font_fixture(id, bytes.clone())?;
        let mut manager = FontManager::default();
        let mut ctx = FontTestContext {
            registered: HashMap::new(),
            reject_registration: false,
        };

        assert_eq!(manager.ensure_font(&mut ctx, &registry, Some(&source), id), Some(id));
        assert_eq!(manager.ensure_font(&mut ctx, &registry, Some(&source), id), Some(id));

        assert_eq!(ctx.registered.get(&id), Some(&bytes.len()));
        Ok(())
    }

    #[test]
    fn ensure_font_uses_default_for_missing_or_invalid_assets() -> Result<(), String> {
        let id = FontId(43);
        let (registry, source) = font_fixture(id, b"invalid font data".to_vec())?;
        let mut manager = FontManager::default();
        let mut ctx = FontTestContext {
            registered: HashMap::new(),
            reject_registration: true,
        };

        assert_eq!(manager.ensure_font(&mut ctx, &registry, Some(&source), id), None);
        assert_eq!(manager.ensure_font(&mut ctx, &registry, Some(&source), FontId(44)), None);
        assert!(ctx.registered.is_empty());
        Ok(())
    }
}
