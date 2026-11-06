use crate::assets::AssetRegistry;
use crate::constants::paths;
use crate::ecs::TomlId;
use crate::storage::{ResourceEntry, ResourceKey, ResourceManifest, ResourceSource};
use crate::text::TextManager;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn prefetched_text_uses_authored_language_and_ui_labels_without_disk() {
    let ui_path = |language: &str| {
        format!("{}/{language}/{}/title.toml", paths::TEXT_FOLDER, paths::UI_TEXT_FOLDER)
    };
    let files = HashMap::from([
        (format!("{}/{}", paths::TEXT_FOLDER, paths::LANGUAGE_MANIFEST),
         b"default_language = 'es'\navailable = ['en', 'es']\n".to_vec()),
        (ui_path(paths::TEXT_LANGUAGE_FOLDER), b"DemoTitle = 'English Title'\n".to_vec()),
        (ui_path("es"), b"DemoTitle = 'Titulo'\n".to_vec()),
        (format!("{}/es/{}/npc.toml", paths::TEXT_FOLDER, paths::DIALOGUE_FOLDER),
         b"[greeting]\nvariants = ['Hola']\n".to_vec()),
    ]);
    let files: HashMap<_, _> = files.into_iter()
        .map(|(path, bytes)| (ResourceKey::from_wire(&path).unwrap(), bytes))
        .collect();
    let mut entries = files.iter().map(|(path, bytes)| ResourceEntry {
        path: path.clone(), byte_len: bytes.len(),
    }).collect::<Vec<_>>();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let source = ResourceSource::from_prefetched(ResourceManifest { files: entries }, files).unwrap();
    let mut manager = TextManager::default();

    manager.set_prefetched_source(PathBuf::from(paths::TEXT_FOLDER), source);

    assert_eq!(manager.get_language(), "es");
    assert_eq!(manager.get_ui_text("ui/title", "DemoTitle"), Some("Titulo".into()));
    let mut registry = AssetRegistry::default();
    registry.register_asset_relative_path(TomlId(1), "dialogue/npc.toml").unwrap();
    assert_eq!(manager.select_text(&registry, TomlId(1), "greeting"), Some("Hola".into()));
    assert!(manager.set_language("en"));
    assert_eq!(manager.get_ui_text("ui/title", "DemoTitle"), Some("English Title".into()));
}
