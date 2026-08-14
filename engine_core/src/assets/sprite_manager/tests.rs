use super::*;
use super::loading::FallbackTextureKind;
use crate::assets::asset_registry::AssetKey;
use crate::assets::AssetRegistry;
use crate::audio::test_utils::CountingFailingLoader;
use crate::constants::paths;
use crate::engine_global::set_game_name;
use crate::hydration::Hydratable;
use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use bishop::prelude::{Texture2D, TextureLoader};
use std::fs;
use std::time::Duration;

struct BranchPanicLoader;

impl TextureLoader for BranchPanicLoader {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        panic!("missing_texture")
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        panic!("path_load")
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("empty_texture")
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }

    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    "non-string panic".to_string()
}

fn fallback_branch_message(sprite_id: SpriteId) -> String {
    let mut sprite_manager = SpriteManager::default();
    fallback_branch_message_for_runtime_state(&mut sprite_manager, sprite_id)
}

fn fallback_branch_message_for_runtime_state(
    sprite_manager: &mut SpriteManager,
    sprite_id: SpriteId,
) -> String {
    let loader = BranchPanicLoader;
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = sprite_manager.get_texture_from_id(&loader, sprite_id);
    }))
    .expect_err("sprite lookup should reach a fallback branch");

    panic_message(panic)
}

fn drain_pending_runtime_reads(
    sprite_manager: &mut SpriteManager,
    loader: &impl TextureLoader,
    sprite_id: SpriteId,
) {
    for _ in 0..100 {
        sprite_manager.poll_pending_texture_reads(loader);
        if !sprite_manager.has_pending_texture_read(sprite_id) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn prewarm_runtime_read_test_case(
    test_name: &str,
    file_name: &str,
) -> (TestGameFolder, SpriteManager, SpriteId) {
    let test_folder = TestGameFolder::new(test_name);
    set_game_name(test_folder.name());

    let mut sprite_manager = SpriteManager::default();
    let file_read_pool = FileReadPool::new();
    let path = PathBuf::from(file_name);
    let sprite_id = SpriteId(7);
    let full_path = assets_folder().join(&path);

    fs::create_dir_all(
        full_path
            .parent()
            .expect("runtime test path should have a parent"),
    )
    .expect("runtime test directory should be writable");
    fs::write(&full_path, [1_u8, 2, 3, 4]).expect("runtime test file should be writable");

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());
    sprite_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
    sprite_manager.enable_runtime_texture_loading_for_test();
    sprite_manager.prewarm_runtime_texture(sprite_id);

    (test_folder, sprite_manager, sprite_id)
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
fn fallback_kind_for_unavailable_sprite_returns_empty_for_zero_id() {
    assert_eq!(
        SpriteManager::fallback_kind_for_unavailable_sprite(SpriteId(0)),
        FallbackTextureKind::Empty
    );
}

#[test]
fn fallback_kind_for_unavailable_sprite_returns_missing_for_nonzero_id() {
    assert_eq!(
        SpriteManager::fallback_kind_for_unavailable_sprite(SpriteId(7)),
        FallbackTextureKind::Missing
    );
}

#[test]
fn get_texture_from_id_uses_empty_fallback_for_zero_id() {
    assert_eq!(fallback_branch_message(SpriteId(0)), "empty_texture");
}

#[test]
fn get_texture_from_id_uses_missing_fallback_for_unknown_nonzero_sprite_id() {
    assert_eq!(fallback_branch_message(SpriteId(9)), "missing_texture");
}

#[test]
fn evict_texture_clears_runtime_texture_state_without_touching_metadata() {
    let mut sprite_manager = SpriteManager::default();
    // `CountingFailingLoader` cannot satisfy `init_texture`, so set up the runtime state directly.
    let sprite_id = SpriteId(7);
    let path = PathBuf::from("sprites/player.png");

    sprite_manager
        .path_to_sprite_id
        .insert(path.clone(), sprite_id);
    sprite_manager
        .sprite_id_to_path
        .insert(sprite_id, path.clone());
    sprite_manager
        .pending_texture_reads
        .insert(sprite_id, path.clone());
    sprite_manager.increment_ref(sprite_id);

    sprite_manager.evict_texture(sprite_id);

    assert_eq!(sprite_manager.texture_count(), 0);
    assert!(!sprite_manager.has_pending_texture_read(sprite_id));
    assert_eq!(sprite_manager.get_or_none(&path), Some(sprite_id));
    assert_eq!(sprite_manager.path_for_id(sprite_id), Some(path.as_path()));
}

#[test]
fn pending_texture_count_tracks_queued_runtime_reads() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let (_folder, sprite_manager, _sprite_id) =
        prewarm_runtime_read_test_case("asset_mgr_pending_count", "textures/runtime-pending-count.bin");

    assert_eq!(sprite_manager.pending_texture_count(), 1);
}

#[test]
fn prewarm_runtime_texture_tracks_pending_sprite_id() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let (_folder, sprite_manager, sprite_id) =
        prewarm_runtime_read_test_case("asset_mgr_queue", "textures/runtime-queue.bin");

    assert!(sprite_manager.has_pending_texture_read(sprite_id));
    assert_eq!(sprite_manager.texture_count(), 0);
}

#[test]
fn get_texture_from_id_uses_empty_fallback_for_pending_runtime_sprite_id() {
    let mut sprite_manager = SpriteManager::default();
    let sprite_id = SpriteId(7);
    let path = PathBuf::from("textures/runtime-pending-fallback.bin");

    sprite_manager.enable_runtime_texture_loading_for_test();
    sprite_manager.path_to_sprite_id.insert(path.clone(), sprite_id);
    sprite_manager.sprite_id_to_path.insert(sprite_id, path.clone());
    sprite_manager.pending_texture_reads.insert(sprite_id, path);

    assert_eq!(
        fallback_branch_message_for_runtime_state(&mut sprite_manager, sprite_id),
        "empty_texture"
    );
}

#[test]
fn get_texture_from_id_uses_missing_fallback_after_runtime_read_failure() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("asset_mgr_failed_runtime_fallback");
    set_game_name(folder.name());

    let mut sprite_manager = SpriteManager::default();
    let file_read_pool = FileReadPool::new();
    let sprite_id = SpriteId(7);
    let path = PathBuf::from("textures/runtime-missing.bin");

    sprite_manager.path_to_sprite_id.insert(path.clone(), sprite_id);
    sprite_manager.sprite_id_to_path.insert(sprite_id, path);
    let loader = CountingFailingLoader::new();
    sprite_manager.attach_runtime_file_read_pool_for_test(&file_read_pool);
    sprite_manager.enable_runtime_texture_loading_for_test();
    sprite_manager.prewarm_runtime_texture(sprite_id);

    drain_pending_runtime_reads(&mut sprite_manager, &loader, sprite_id);

    assert_eq!(
        fallback_branch_message_for_runtime_state(&mut sprite_manager, sprite_id),
        "missing_texture"
    );
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
    sprite_manager.prewarm_runtime_texture(sprite_id);

    // Drain until the read completes and the upload path is hit
    drain_pending_runtime_reads(&mut sprite_manager, &loader, sprite_id);

    assert_eq!(loader.bytes_load_calls.get(), 1);
    assert!(!sprite_manager.has_pending_texture_read(sprite_id));
}

#[test]
fn evict_texture_refuses_when_ref_count_positive() {
    let mut sm = SpriteManager::default();
    sm.increment_ref(SpriteId(1));
    sm.increment_ref(SpriteId(1));
    sm.evict_texture(SpriteId(1));
    assert_eq!(sm.ref_count(&SpriteId(1)), 1);
}

#[test]
fn evict_texture_succeeds_when_ref_count_zero() {
    let mut sm = SpriteManager::default();
    let result = sm.evict(&SpriteId(1));
    assert_eq!(result, Ok(()));
}

#[test]
fn ref_count_increments_and_decrements_correctly() {
    let mut sm = SpriteManager::default();
    sm.increment_ref(SpriteId(1));
    assert_eq!(sm.ref_count(&SpriteId(1)), 1);
    sm.increment_ref(SpriteId(1));
    assert_eq!(sm.ref_count(&SpriteId(1)), 2);
    sm.decrement_ref(SpriteId(1));
    assert_eq!(sm.ref_count(&SpriteId(1)), 1);
}
