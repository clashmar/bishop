use super::*;
use crate::assets::asset_registry::AssetKey;
use crate::assets::AssetRegistry;
use crate::constants::paths;
use crate::engine_global::set_game_name;
use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::cell::Cell;
use std::fs;
use std::time::Duration;

struct CountingFailingLoader {
    bytes_load_calls: Cell<usize>,
    load_calls: Cell<usize>,
}

impl CountingFailingLoader {
    fn new() -> Self {
        Self {
            bytes_load_calls: Cell::new(0),
            load_calls: Cell::new(0),
        }
    }
}

impl TextureLoader for CountingFailingLoader {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        self.bytes_load_calls
            .set(self.bytes_load_calls.get().saturating_add(1));
        Err("expected test byte load failure".to_string())
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        self.load_calls.set(self.load_calls.get() + 1);
        Err("expected test load failure".to_string())
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("empty_texture is not used in asset manager tests")
    }
}

#[test]
fn get_or_load_registers_new_sprite_path_in_asset_registry() {
    let loader = CountingFailingLoader::new();
    let mut registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let path = PathBuf::from("sprites/player.png");

    let result = sprite_manager.get_or_load(&mut registry, &loader, &path);

    assert!(result.is_none());
    assert_eq!(
        registry.key_for_path(PathBuf::from(paths::ASSETS_FOLDER).join(&path)),
        Some(AssetKey::Sprite(SpriteId(1)))
    );
    assert_eq!(sprite_manager.get_or_none(&path), Some(SpriteId(1)));
}

#[test]
fn get_or_load_retries_loader_for_registered_path_with_missing_texture() {
    let loader = CountingFailingLoader::new();
    let mut registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let path = PathBuf::from("sprites/player.png");
    let sprite_id = SpriteId(7);

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());

    let result = sprite_manager.get_or_load(&mut registry, &loader, &path);

    assert!(result.is_none());
    assert_eq!(loader.load_calls.get(), 1);
}

#[test]
fn init_editor_metadata_rebuilds_sprite_cache_from_asset_registry() {
    let mut registry = AssetRegistry::default();
    registry
        .register_asset_relative_path(SpriteId(1), "sprites/player.png")
        .expect("sprite path should register");

    let mut sprite_manager = SpriteManager::default();
    SpriteManager::init_editor_metadata(&registry, &mut sprite_manager);

    assert_eq!(
        sprite_manager
            .path_for_id(SpriteId(1))
            .map(|path| path.to_path_buf()),
        Some(PathBuf::from("sprites/player.png"))
    );
    assert_eq!(
        sprite_manager.get_or_none("sprites/player.png"),
        Some(SpriteId(1))
    );
}

#[test]
fn ensure_loaded_retries_loader_for_registered_sprite_id_with_missing_texture() {
    let loader = CountingFailingLoader::new();
    let mut sprite_manager = SpriteManager::default();
    let path = PathBuf::from("sprites/player.png");
    let sprite_id = SpriteId(7);

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());

    let result = sprite_manager.ensure_loaded(&loader, sprite_id);

    assert!(result.is_err());
    assert_eq!(loader.load_calls.get(), 1);
}

#[test]
fn queue_runtime_texture_read_tracks_pending_sprite_id() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("asset_mgr_queue");
    set_game_name(test_folder.name());

    let mut sprite_manager = SpriteManager::default();
    let file_read_pool = FileReadPool::new();
    let path = PathBuf::from("textures/runtime-queue.bin");
    let sprite_id = SpriteId(7);
    let full_path = assets_folder().join(&path);

    fs::create_dir_all(
        full_path
            .parent()
            .expect("runtime queue test path should have a parent"),
    )
    .expect("runtime queue test directory should be writable");
    fs::write(&full_path, [1_u8, 2, 3, 4]).expect("runtime queue test file should be writable");

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());
    sprite_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
    sprite_manager.enable_runtime_texture_loading_for_test();
    sprite_manager.queue_runtime_texture_read(sprite_id);

    assert!(sprite_manager.has_pending_texture_read(sprite_id));
    assert_eq!(sprite_manager.texture_count(), 0);
}

#[test]
fn poll_pending_runtime_texture_reads_uploads_bytes_on_the_main_thread() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("asset_mgr_upload");
    set_game_name(test_folder.name());

    let loader = CountingFailingLoader::new();
    let mut sprite_manager = SpriteManager::default();
    let file_read_pool = FileReadPool::new();
    let path = PathBuf::from("textures/runtime-upload.bin");
    let sprite_id = SpriteId(7);
    let full_path = assets_folder().join(&path);

    fs::create_dir_all(
        full_path
            .parent()
            .expect("runtime upload test path should have a parent"),
    )
    .expect("runtime upload test directory should be writable");
    fs::write(&full_path, [1, 2, 3, 4]).expect("runtime upload test file should be writable");

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());
    sprite_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
    sprite_manager.enable_runtime_texture_loading_for_test();
    sprite_manager.queue_runtime_texture_read(sprite_id);

    // Drain until the read completes and the upload path is hit.
    for _ in 0..100 {
        sprite_manager.poll_pending_texture_reads(&loader);
        if !sprite_manager.has_pending_texture_read(sprite_id) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(loader.bytes_load_calls.get(), 1);
    assert!(!sprite_manager.has_pending_texture_read(sprite_id));
}
