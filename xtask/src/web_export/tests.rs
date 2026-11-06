use std::path::Path;

use engine_core::constants::paths;
use engine_core::ecs::Sprite;
use engine_core::storage::{load_full_game_from_folder, web_game_export::collect_game_resources};

#[test]
fn second_authored_game_has_its_own_player_world_title_and_script() -> Result<(), String> {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/web_smoke/Resources");
    let game = load_full_game_from_folder(&resources).map_err(|error| error.to_string())?;
    assert_eq!(game.name, "WebSmoke");
    assert_eq!(game.worlds().len(), 1);
    assert_eq!(game.worlds()[0].name, "Lantern Workshop");
    let player = game.ecs.get_player_entity().ok_or("missing authored player")?;
    assert!(game.ecs.get::<Sprite>(player).is_some());
    let manifest = collect_game_resources(&resources)?;
    for path in ["startup.ron", "menus/title.ron", "scripts/main.lua",
                 "assets/sprites/smoke.png"] {
        assert!(manifest.files.iter().any(|entry| entry.path.as_str() == path), "missing {path}");
    }
    let script = std::fs::read_to_string(resources.join("scripts/main.lua"))
        .map_err(|error| error.to_string())?;
    assert!(script.contains("WEB_SMOKE_LANTERN_LIT"));
    Ok(())
}

#[test]
fn web_export_fixture_manifest_uses_game_resources() -> Result<(), String> {
    let root = Path::new("/test/workspace");
    let url = web_shell::FIXTURE_MANIFEST_PATH;
    let relative = url
        .strip_prefix(web_shell::FIXTURE_ASSET_ROOT)
        .ok_or_else(|| format!("manifest URL outside fixture root: {url}"))?;
    assert_eq!(
        super::fixture_source(root, url),
        Ok(root
            .join(paths::GAME_SAVE_ROOT)
            .join(web_shell::FIXTURE_NAME)
            .join(paths::RESOURCES_FOLDER)
            .join(relative))
    );
    Ok(())
}

#[test]
fn web_export_fixture_modules_use_game_scripts() -> Result<(), String> {
    let root = Path::new("/test/workspace");
    for name in web_shell::FIXTURE_MODULES {
        let url = web_shell::fixture_module_path(name);
        let relative = url
            .strip_prefix(web_shell::FIXTURE_ASSET_ROOT)
            .ok_or_else(|| format!("module URL outside fixture root: {url}"))?;
        assert_eq!(
            super::fixture_source(root, &url),
            Ok(root
                .join(paths::GAME_SAVE_ROOT)
                .join(web_shell::FIXTURE_NAME)
                .join(paths::RESOURCES_FOLDER)
                .join(relative))
        );
    }
    Ok(())
}

#[test]
fn web_export_playable_build_uses_an_isolated_target_directory() {
    let root = Path::new("/test/workspace");
    assert_eq!(
        super::web_game_build_dir(root),
        root.join("target")
            .join("web-game-build")
            .join(super::WEB_WASM_TARGET)
            .join("debug")
    );
}

#[test]
fn web_player_build_uses_an_independent_release_artifact() {
    let root = Path::new("/test/workspace");
    assert_eq!(super::web_player_build_dir(root), root.join("target/web-player-build")
        .join(super::WEB_WASM_TARGET).join("release"));
    assert_ne!(super::web_player_build_dir(root), super::web_demo_build_dir(root));
}

#[test]
fn web_game_export_requires_explicit_resources_and_output() {
    let args = ["--resources", "games/Other/Resources", "--out", "/tmp/site"]
        .map(str::to_owned);
    assert_eq!(super::parse_export_web_game_args(&args),
        Ok(("games/Other/Resources".into(), "/tmp/site".into())));
    for invalid in [
        vec!["--resources", "games/Other/Resources"],
        vec!["--out", "/tmp/site", "--unknown", "value"],
        vec!["--resources", "", "--out", "/tmp/site"],
        vec!["--resources", "one", "--resources", "two", "--out", "/tmp/site"],
    ] {
        assert!(super::parse_export_web_game_args(&invalid.iter().map(|s| s.to_string())
            .collect::<Vec<_>>()).is_err(), "{invalid:?}");
    }
}

#[test]
fn web_export_rejects_urls_outside_fixture_root() {
    let root = Path::new("/test/workspace");
    assert!(super::fixture_source(root, web_shell::DEMO_TITLE_ASSET_PATH).is_err());
}

#[test]
fn web_export_renderer_assets_resolve_inside_demo_resources() -> Result<(), String> {
    let root = Path::new("/test/workspace");
    let png = web_shell::DEMO_RENDERER_SHEET_PATH;
    let atlas = web_shell::demo_renderer_atlas_path();
    assert_eq!(atlas, "assets/demo/assets/animations/player/male/Idle.json");
    for url in [png, atlas.as_str()] {
        let relative = url
            .strip_prefix(web_shell::DEMO_RENDERER_ASSET_ROOT)
            .ok_or_else(|| format!("renderer URL outside Demo asset root: {url}"))?;
        assert_eq!(super::renderer_source(root, url), Ok(super::demo_resources_dir(root).join(relative)));
    }
    Ok(())
}

#[test]
fn web_export_renderer_rejects_outside_urls_and_traversal() {
    let root = Path::new("/test/workspace");
    assert!(super::renderer_source(root, web_shell::FIXTURE_MANIFEST_PATH).is_err());
    assert!(super::renderer_source(root, "assets/demo/../other.png").is_err());
}

#[test]
fn web_export_renderer_uses_separate_build_and_stage_directories() {
    let root = Path::new("/test/workspace");
    assert_eq!(super::web_renderer_build_dir(root),
        root.join("target").join("web-renderer-build").join(super::WEB_WASM_TARGET).join("debug"));
    assert_eq!(super::web_renderer_stage_dir(root), root.join("target").join("web-renderer-proof"));
    assert_ne!(super::web_renderer_stage_dir(root), super::web_game_stage_dir(root));
}

#[test]
fn web_export_renderer_missing_source_rejects_stage() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    assert!(!super::stage_file(
        &temp.path().join("missing"),
        &temp.path().join(web_shell::DEMO_RENDERER_SHEET_PATH),
        "web renderer resource"
    ));
    Ok(())
}

#[test]
fn web_export_missing_required_source_rejects_stage() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    assert!(!super::stage_file(
        &temp.path().join("missing"),
        &temp.path().join("output"),
        "web fixture resource"
    ));
    Ok(())
}
