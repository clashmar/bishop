use bishop::prelude::*;
use engine_core::prelude::*;
use ron::de::from_str;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
struct PlaytestPayloadPreview {
    game: PlaytestGamePreview,
}

#[derive(serde::Deserialize)]
struct PlaytestGamePreview {
    name: String,
}

/// Builds a runtime icon from PNG bytes.
pub fn load_icon_from_png_bytes(png_bytes: &[u8]) -> WindowIcon {
    WindowIcon::Rgba {
        small: Some(IconData::new(
            load_rgba_resized::<{ 16 * 16 * 4 }>(png_bytes, 16).to_vec(),
            16,
            16,
        )),
        medium: Some(IconData::new(
            load_rgba_resized::<{ 32 * 32 * 4 }>(png_bytes, 32).to_vec(),
            32,
            32,
        )),
        large: Some(IconData::new(
            load_rgba_resized::<{ 64 * 64 * 4 }>(png_bytes, 64).to_vec(),
            64,
            64,
        )),
    }
}

/// Loads the runtime icon from a resources directory.
pub fn runtime_icon_from_resources_dir(resources_dir: &Path) -> Option<WindowIcon> {
    let icon_path = resources_dir.join("Icon.png");
    fs::read(icon_path)
        .ok()
        .map(|png_bytes| load_icon_from_png_bytes(&png_bytes))
}

/// Loads the runtime icon for the current game executable.
pub fn runtime_icon_for_current_exe() -> Option<WindowIcon> {
    let resources_dir = resources_dir_from_exe()?;
    runtime_icon_from_resources_dir(&resources_dir)
}

/// Extracts the game name from a playtest payload.
pub fn playtest_game_name_from_payload(payload_ron: &str) -> Option<String> {
    let preview = from_str::<PlaytestPayloadPreview>(payload_ron).ok()?;
    Some(preview.game.name)
}

/// Loads the runtime icon for a playtest payload.
pub fn runtime_icon_for_playtest_payload(payload_path: &str) -> Option<WindowIcon> {
    let payload_ron = fs::read_to_string(payload_path).ok()?;
    let game_name = playtest_game_name_from_payload(&payload_ron)?;
    let resources_dir = game_folder(&game_name).join(RESOURCES_FOLDER);
    runtime_icon_from_resources_dir(&resources_dir)
}
