/// Browser-relative path for the staged Demo title asset.
pub const DEMO_TITLE_ASSET_PATH: &str = "assets/demo/text/en/ui/title.toml";

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
