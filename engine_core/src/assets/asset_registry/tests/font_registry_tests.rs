use crate::assets::asset_registry::{AssetKey, AssetKind, AssetRegistry};
use crate::constants::extensions;
use crate::constants::paths::FONTS_FOLDER;
use bishop::text::FontId;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[test]
fn font_key_uses_canonical_folder_and_resolves_by_relative_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = AssetRegistry::default();
    let id = FontId(3);
    let file_name = format!("Atkinson-Hyperlegible.{}", extensions::TTF);
    let relative_path = PathBuf::from(FONTS_FOLDER).join(&file_name);

    assert_eq!(AssetRegistry::kind_for_key(AssetKey::Font(id)), AssetKind::Font);
    assert_eq!(
        AssetRegistry::asset_folder(AssetKind::Font),
        PathBuf::from(FONTS_FOLDER)
    );

    registry.register_asset_relative_path(id, &file_name)?;

    assert_eq!(
        registry.relative_path(id),
        Some(PathBuf::from(file_name))
    );
    assert_eq!(registry.key_for_path(relative_path), Some(AssetKey::Font(id)));
    Ok(())
}

#[test]
fn supported_extension_infers_font_kind() {
    let font_path = PathBuf::from(format!("display.{}", extensions::OTF));
    assert_eq!(AssetKind::from_path(&font_path), Some(AssetKind::Font));
    assert_eq!(AssetKind::from_path(Path::new("unknown.woff")), None);
}

#[test]
fn font_registry_accepts_ttf_and_otf_but_rejects_unsupported_extensions(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = AssetRegistry::default();
    registry.register_asset_relative_path(
        FontId(1),
        format!("body.{}", extensions::TTF),
    )?;
    registry.register_asset_relative_path(
        FontId(2),
        format!("display.{}", extensions::OTF),
    )?;

    let error = registry
        .register_asset_relative_path(FontId(3), "webfont.woff")
        .err()
        .ok_or_else(|| std::io::Error::other("unsupported font extension was accepted"))?;

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(registry.records().len(), 2);
    Ok(())
}
