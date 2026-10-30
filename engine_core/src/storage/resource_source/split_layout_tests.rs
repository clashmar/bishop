use crate::audio::AudioManager;
use crate::audio::test_utils::TestBackend;
use crate::constants::paths;
use crate::ecs::SpriteId;
use crate::hydration::{HydrationDriver, ResidencyKey, ScopeKey};
use mlua::Lua;
use crate::game::Game;
use crate::storage::{
    ResourceEntry, ResourceManifest, ResourceSource, load_game_shell_from_source,
    hydrate_current_payloads_from_source, room_payload_path, save_game_to_folder,
    world_descriptor_path,
};
use crate::worlds::{Room, RoomId, World, WorldId};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path};
use tempfile::TempDir;

fn saved_one_room_game(root: &Path) -> (WorldId, RoomId) {
    let mut game = Game::with_name("source_fixture");
    let (world_id, room_id) = (WorldId(1), RoomId(2));
    let mut world = World::new(world_id, "fixture".into(), 16.0);
    world.current_room_id = Some(room_id);
    world.add_room(Room::new(&mut game.ecs, room_id, 16.0));
    game.add_world(world);
    game.current_world_id = Some(world_id);
    game.asset_registry.register_asset_relative_path(SpriteId(3), "sprite.png").unwrap();
    save_game_to_folder(&game, root).unwrap();
    (world_id, room_id)
}

fn manifest_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap().components().map(|component| {
        let Component::Normal(part) = component else { panic!("generated path escaped root") };
        part.to_str().unwrap()
    }).collect::<Vec<_>>().join("/")
}

fn split_bytes(root: &Path, world_id: WorldId, room_id: RoomId) -> (ResourceManifest, HashMap<String, Vec<u8>>) {
    let mut files = HashMap::new();
    let paths = [
        root.join(paths::GAME_RON),
        root.join(paths::ECS_RON),
        root.join(paths::ASSET_REGISTRY_RON),
        root.join(paths::TILE_REGISTRY_RON),
        world_descriptor_path(root, world_id),
        room_payload_path(root, room_id),
    ];
    for path in paths {
        files.insert(manifest_key(root, &path), fs::read(path).unwrap());
    }
    let mut entries: Vec<_> = files.iter().map(|(path, bytes)| ResourceEntry {
        path: path.clone(), byte_len: bytes.len(),
    }).collect();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    (ResourceManifest { files: entries }, files)
}

#[test]
fn split_layout_when_loaded_from_bytes_matches_folder_shell_and_payload() {
    let root = TempDir::new().unwrap();
    let (world_id, room_id) = saved_one_room_game(root.path());
    let (manifest, files) = split_bytes(root.path(), world_id, room_id);
    let bytes = ResourceSource::from_prefetched(manifest, files).unwrap();
    let folder = ResourceSource::Folder(root.path().to_path_buf());

    let mut disk_game = load_game_shell_from_source(&folder).unwrap();
    let mut byte_game = load_game_shell_from_source(&bytes).unwrap();
    hydrate_current_payloads_from_source(&folder, &mut disk_game).unwrap();
    hydrate_current_payloads_from_source(&bytes, &mut byte_game).unwrap();

    assert_eq!(disk_game.name, byte_game.name);
    assert_eq!(disk_game.current_world().current_room_id, byte_game.current_world().current_room_id);
    assert_eq!(disk_game.current_world().current_room().unwrap().variants.len(),
        byte_game.current_world().current_room().unwrap().variants.len());
    assert_eq!(ron::to_string(&disk_game.ecs).unwrap(), ron::to_string(&byte_game.ecs).unwrap());
    assert_eq!(ron::to_string(&disk_game.asset_registry).unwrap(),
        ron::to_string(&byte_game.asset_registry).unwrap());
    assert_eq!(ron::to_string(&disk_game.tile_registry).unwrap(),
        ron::to_string(&byte_game.tile_registry).unwrap());
}

#[test]
fn split_layout_when_descriptor_is_missing_reports_its_path() {
    let root = TempDir::new().unwrap();
    let (world_id, room_id) = saved_one_room_game(root.path());
    let (mut manifest, mut files) = split_bytes(root.path(), world_id, room_id);
    let missing = world_descriptor_path(root.path(), world_id);
    let key = manifest_key(root.path(), &missing);
    manifest.files.retain(|entry| entry.path != key);
    files.remove(&key);
    let bytes = ResourceSource::from_prefetched(manifest, files).unwrap();

    let error = load_game_shell_from_source(&bytes).err().unwrap();
    assert!(error.to_string().contains(&key), "{error}");
}

#[test]
fn deferred_room_hydration_uses_retained_bytes_without_the_source_folder() {
    let root = TempDir::new().unwrap();
    let (world_id, room_id) = saved_one_room_game(root.path());
    let (manifest, files) = split_bytes(root.path(), world_id, room_id);
    let bytes = ResourceSource::from_prefetched(manifest, files).unwrap();
    let mut game = load_game_shell_from_source(&bytes).unwrap();
    assert!(game.current_world().current_room().unwrap().variants.is_empty());
    drop(root);

    let mut audio_manager = AudioManager::new::<TestBackend>();
    let mut driver = HydrationDriver { game: &mut game, audio_manager: &mut audio_manager };
    driver.hydrate_key_runtime(ResidencyKey::Scope(ScopeKey::Room(room_id)), &Lua::new()).unwrap();

    assert!(!driver.game.current_world().current_room().unwrap().variants.is_empty());
}

#[test]
fn current_payload_when_ron_is_malformed_reports_its_path() {
    let root = TempDir::new().unwrap();
    let (world_id, room_id) = saved_one_room_game(root.path());
    let (mut manifest, mut files) = split_bytes(root.path(), world_id, room_id);
    let payload = room_payload_path(root.path(), room_id);
    let key = manifest_key(root.path(), &payload);
    let broken = b"not valid RON".to_vec();
    files.insert(key.clone(), broken.clone());
    manifest.files.iter_mut().find(|entry| entry.path == key).unwrap().byte_len = broken.len();
    let bytes = ResourceSource::from_prefetched(manifest, files).unwrap();
    let mut game = load_game_shell_from_source(&bytes).unwrap();

    let error = hydrate_current_payloads_from_source(&bytes, &mut game).unwrap_err();
    assert!(error.to_string().contains(&key), "{error}");
}
