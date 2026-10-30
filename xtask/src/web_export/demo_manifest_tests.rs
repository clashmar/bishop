use super::demo_manifest::collect_demo_resources;
use engine_core::constants::paths;
use engine_core::ecs::{SpriteId, TomlId};
use engine_core::game::Game;
use engine_core::storage::{load_full_game_from_folder, room_payload_path, save_game_to_folder, world_descriptor_path};
use engine_core::worlds::{Room, RoomId, World, WorldId};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn one_room_export_tree(root: &Path, reverse: bool) -> (WorldId, RoomId) {
    let mut game = Game::with_name("export_fixture");
    let (world_id, room_id) = (WorldId(7), RoomId(9));
    let mut world = World::new(world_id, "fixture".into(), 16.0);
    world.current_room_id = Some(room_id);
    world.add_room(Room::new(&mut game.ecs, room_id, 16.0));
    game.add_world(world);
    game.current_world_id = Some(world_id);
    game.asset_registry.register_asset_relative_path(SpriteId(1), "player.png").unwrap();
    save_game_to_folder(&game, root).unwrap();

    let script = root.join(paths::SCRIPTS_FOLDER).join("main.lua");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(script, "return {}").unwrap();
    let files = [
        ("player.png", b"png".as_slice()),
        ("player.aseprite", b"source".as_slice()),
    ];
    let assets = root.join(paths::ASSETS_FOLDER);
    fs::create_dir_all(&assets).unwrap();
    let ordered: Vec<_> = if reverse { files.iter().rev().collect() } else { files.iter().collect() };
    for (path, contents) in ordered {
        fs::write(assets.join(path), contents).unwrap();
    }
    (world_id, room_id)
}

#[test]
fn export_when_files_are_created_in_different_order_has_same_manifest() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    one_room_export_tree(first.path(), false);
    one_room_export_tree(second.path(), true);

    let left = collect_demo_resources(first.path()).unwrap();
    let right = collect_demo_resources(second.path()).unwrap();

    assert_eq!(ron::to_string(&left).unwrap(), ron::to_string(&right).unwrap());
    assert!(!left.files.iter().any(|entry| entry.path.ends_with(".aseprite")));
    assert!(left.files.iter().any(|entry| entry.path == paths::GAME_RON));
}

#[test]
fn export_when_declared_world_descriptor_is_missing_reports_path() {
    let root = TempDir::new().unwrap();
    let (world_id, _) = one_room_export_tree(root.path(), false);
    let descriptor = world_descriptor_path(root.path(), world_id);
    fs::remove_file(&descriptor).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(&descriptor.display().to_string()), "{error}");
}

#[test]
fn export_when_declared_room_payload_is_missing_reports_path() {
    let root = TempDir::new().unwrap();
    let (_, room_id) = one_room_export_tree(root.path(), false);
    let payload = room_payload_path(root.path(), room_id);
    fs::remove_file(&payload).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(&payload.display().to_string()), "{error}");
}

#[test]
fn export_when_registered_asset_is_missing_reports_path() {
    let root = TempDir::new().unwrap();
    one_room_export_tree(root.path(), false);
    let image = root.path().join(paths::ASSETS_FOLDER).join("player.png");
    fs::remove_file(&image).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(&image.display().to_string()), "{error}");
}

#[test]
fn export_when_game_manifest_is_missing_reports_path() {
    let root = TempDir::new().unwrap();
    one_room_export_tree(root.path(), false);
    fs::remove_file(root.path().join(paths::GAME_RON)).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(paths::GAME_RON), "{error}");
}

fn register_language_neutral_text(root: &Path) -> std::path::PathBuf {
    let mut game = load_full_game_from_folder(root).unwrap();
    game.asset_registry
        .register_asset_relative_path(TomlId(2), Path::new("dialogue").join("npc.toml"))
        .unwrap();
    save_game_to_folder(&game, root).unwrap();
    let physical = root.join(paths::TEXT_FOLDER).join(paths::TEXT_LANGUAGE_FOLDER)
        .join("dialogue").join("npc.toml");
    fs::create_dir_all(physical.parent().unwrap()).unwrap();
    fs::write(&physical, "[npc]\nname = 'fixture'\n").unwrap();
    physical
}

#[test]
fn export_when_text_registry_is_language_neutral_stages_language_file() {
    let root = TempDir::new().unwrap();
    one_room_export_tree(root.path(), false);
    let physical = register_language_neutral_text(root.path());

    let manifest = collect_demo_resources(root.path()).unwrap();
    let relative = physical.strip_prefix(root.path()).unwrap();
    assert!(manifest.files.iter().any(|entry| Path::new(&entry.path) == relative));
}

#[test]
fn export_when_resolved_text_file_is_missing_reports_physical_path() {
    let root = TempDir::new().unwrap();
    one_room_export_tree(root.path(), false);
    let physical = register_language_neutral_text(root.path());
    fs::remove_file(&physical).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(&physical.display().to_string()), "{error}");
}

#[cfg(unix)]
#[test]
fn export_when_resource_is_a_symlink_rejects_it() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    one_room_export_tree(root.path(), false);
    let link = root.path().join(paths::SCRIPTS_FOLDER).join("external.lua");
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();

    let error = collect_demo_resources(root.path()).unwrap_err();
    assert!(error.contains(&link.display().to_string()), "{error}");
}
