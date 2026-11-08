use super::web_site_export::stage_web_site;
use crate::game::Game;
use crate::storage::save_game_to_folder;
use crate::worlds::{Room, RoomId, World, WorldId};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn authored_resources(root: &Path) {
    let mut game = Game::with_name("Other Game");
    let mut world = World::new(WorldId(2), "Other World".into(), 16.0);
    let room_id = RoomId(3);
    world.current_room_id = Some(room_id);
    world.add_room(Room::new(&mut game.ecs, room_id, 16.0));
    game.add_world(world);
    game.current_world_id = Some(WorldId(2));
    save_game_to_folder(&game, root).unwrap();
}

fn fake_player_runtime(root: &Path) {
    fs::create_dir_all(root.join("player")).unwrap();
    for name in ["player.html", "input.js", "audio.js", "web_shell.js", "web_shell.wasm",
                 "player/config.mjs", "player/save_store.mjs"] {
        fs::write(root.join(name), format!("runtime asset: {name}")).unwrap();
    }
    fs::write(root.join("index-template.html"),
        "<button>Reveal</button><script>const game = '{{GAME_PATH}}';</script>").unwrap();
}

#[test]
fn web_site_stages_one_reusable_player_and_a_separate_named_game() {
    let source = TempDir::new().unwrap();
    let runtime = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    authored_resources(source.path());
    fake_player_runtime(runtime.path());
    let site = output.path().join("site");

    let layout = stage_web_site(source.path(), runtime.path(), &site).unwrap();

    assert_eq!(layout.game_dir_name, "Other Game");
    assert_eq!(fs::read(site.join("bishop-player/web_shell.wasm")).unwrap(),
               fs::read(runtime.path().join("web_shell.wasm")).unwrap());
    assert_eq!(fs::read(site.join("games/Other Game/game.ron")).unwrap(),
               fs::read(source.path().join("game.ron")).unwrap());
    assert!(site.join("games/Other Game/manifest.ron").is_file());
    assert!(site.join("bishop-player/player/config.mjs").is_file());
    assert!(site.join("bishop-player/player/save_store.mjs").is_file());
    assert!(fs::read_to_string(site.join("index.html")).unwrap()
        .contains("../games/Other%20Game/manifest.ron"));
}

#[test]
fn web_site_preserves_an_existing_destination() {
    let source = TempDir::new().unwrap();
    let runtime = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    authored_resources(source.path());
    fake_player_runtime(runtime.path());
    let site = output.path().join("site");
    fs::create_dir(&site).unwrap();
    fs::write(site.join("keep.txt"), "existing export").unwrap();

    let error = stage_web_site(source.path(), runtime.path(), &site).unwrap_err();

    assert!(error.contains(&site.display().to_string()), "{error}");
    assert_eq!(fs::read_to_string(site.join("keep.txt")).unwrap(), "existing export");
}

#[test]
fn web_site_requires_prebuilt_runtime_before_creating_output() {
    let source = TempDir::new().unwrap();
    let runtime = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    authored_resources(source.path());
    let site = output.path().join("site");

    let error = stage_web_site(source.path(), runtime.path(), &site).unwrap_err();

    assert!(error.contains("player.html"), "{error}");
    assert!(!site.exists());
}

#[test]
fn web_site_requires_browser_save_module_before_creating_output() {
    let source = TempDir::new().unwrap();
    let runtime = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    authored_resources(source.path());
    fake_player_runtime(runtime.path());
    let module = runtime.path().join("player/save_store.mjs");
    fs::remove_file(&module).unwrap();
    let site = output.path().join("site");

    let error = stage_web_site(source.path(), runtime.path(), &site).unwrap_err();

    assert!(error.contains(&module.display().to_string()), "{error}");
    assert!(!site.exists());
}

#[test]
fn web_site_rejects_output_inside_authored_resources_without_writing() {
    let source = TempDir::new().unwrap();
    let runtime = TempDir::new().unwrap();
    authored_resources(source.path());
    fake_player_runtime(runtime.path());
    let assets = source.path().join("assets");
    fs::create_dir_all(&assets).unwrap();
    let site = assets.join("site");

    let error = stage_web_site(source.path(), runtime.path(), &site).unwrap_err();

    assert!(error.contains("inside authored Resources"), "{error}");
    assert!(!site.exists());
}
