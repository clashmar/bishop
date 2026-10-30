/// Browser-relative path for the staged Demo title asset.
pub const DEMO_TITLE_ASSET_PATH: &str = "assets/demo/text/en/ui/title.toml";

/// URL prefix for authored Demo resources in the isolated WebGL2 renderer proof.
pub const DEMO_RENDERER_ASSET_ROOT: &str = "assets/demo/";
/// Staged browser URL for the authored Demo player animation sheet.
pub const DEMO_RENDERER_SHEET_PATH: &str = "assets/demo/assets/animations/player/male/Idle.png";

/// Returns the adjacent Demo animation atlas URL without pinning frame pixel data.
pub fn demo_renderer_atlas_path() -> String {
    format!("{}.json", DEMO_RENDERER_SHEET_PATH.trim_end_matches(".png"))
}

/// URL prefix for staged authored Demo checkpoint resources.
pub const DEMO_CHECKPOINT_ASSET_ROOT: &str = DEMO_RENDERER_ASSET_ROOT;
/// Name of the source-derived browser resource manifest.
pub const DEMO_CHECKPOINT_MANIFEST_FILE: &str = "manifest.ron";

/// Returns the URL of the staged Demo checkpoint resource manifest.
pub fn demo_checkpoint_manifest_path() -> String {
    format!("{DEMO_CHECKPOINT_ASSET_ROOT}{DEMO_CHECKPOINT_MANIFEST_FILE}")
}

/// Browser fixture project directory under `games/`.
pub const FIXTURE_NAME: &str = "BrowserFixture";
/// URL prefix for the dedicated playable fixture's resources.
pub const FIXTURE_ASSET_ROOT: &str = "assets/fixture/";
/// Entry module of the browser fixture's authored Lua rules.
pub const FIXTURE_ENTRY_MODULE: &str = "fixture.main";
/// All authored modules required before synchronous Lua `require` is called.
pub const FIXTURE_MODULES: [&str; 2] = [FIXTURE_ENTRY_MODULE, "fixture.rules"];
/// Staged browser URL for the Bishop game-data manifest.
pub const FIXTURE_MANIFEST_PATH: &str = "assets/fixture/game.ron";

/// Returns the staged URL for one fixture Lua module.
pub fn fixture_module_path(name: &str) -> String {
    format!("{FIXTURE_ASSET_ROOT}scripts/{}.lua", name.replace('.', "/"))
}
