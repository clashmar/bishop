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
fn web_export_missing_required_source_rejects_stage() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    assert!(!super::stage_file(
        &temp.path().join("missing"),
        &temp.path().join("output"),
        "web fixture resource"
    ));
    Ok(())
}
