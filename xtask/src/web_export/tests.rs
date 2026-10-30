use std::path::Path;

use engine_core::constants::paths;

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
