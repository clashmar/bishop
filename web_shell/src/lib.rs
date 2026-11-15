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
