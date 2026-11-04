//! Compile-free static site assembly shared by the editor and xtask.

use crate::storage::load_full_game_from_folder;
use crate::storage::web_game_bundle::WEB_GAME_MANIFEST_FILE;
use crate::storage::web_game_export::{ensure_export_destination, stage_web_game_resources};
use std::fs;
use std::path::Path;

pub const PLAYER_DIRECTORY: &str = "bishop-player";
pub const GAMES_DIRECTORY: &str = "games";
pub const INDEX_TEMPLATE_FILE: &str = "index-template.html";
pub const PLAYER_FILES: [&str; 6] = [
    "player.html", "input.js", "audio.js", "web_shell.js", "web_shell.wasm", "player/config.mjs",
];

#[derive(Debug, PartialEq, Eq)]
pub struct WebSiteLayout {
    pub game_dir_name: String,
}

/// Creates a new site containing a prebuilt reusable player and one authored game bundle.
/// No compiler is invoked; an existing destination is never overwritten.
pub fn stage_web_site(
    resources: &Path,
    runtime: &Path,
    destination: &Path,
) -> Result<WebSiteLayout, String> {
    let game = load_full_game_from_folder(resources)
        .map_err(|error| format!("invalid authored game in {}: {error}", resources.display()))?;
    let game_dir_name = checked_game_directory(&game.name)?;
    ensure_export_destination(resources, destination)?;
    require_directory(runtime)?;
    for file in PLAYER_FILES {
        require_file(&runtime.join(file))?;
    }
    let template_path = runtime.join(INDEX_TEMPLATE_FILE);
    require_file(&template_path)?;
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("failed to read {}: {error}", template_path.display()))?;
    if template.matches("{{GAME_PATH}}").count() != 1 {
        return Err(format!("{} must contain exactly one {{{{GAME_PATH}}}} placeholder", template_path.display()));
    }

    fs::create_dir(destination)
        .map_err(|error| format!("failed to create new export {}: {error}", destination.display()))?;
    let result = (|| {
        let games = destination.join(GAMES_DIRECTORY);
        fs::create_dir(&games)
            .map_err(|error| format!("failed to create {}: {error}", games.display()))?;
        stage_web_game_resources(resources, &games.join(&game_dir_name))?;

        let player = destination.join(PLAYER_DIRECTORY);
        fs::create_dir(&player)
            .map_err(|error| format!("failed to create {}: {error}", player.display()))?;
        for file in PLAYER_FILES {
            let from = runtime.join(file);
            let to = player.join(file);
            let parent = to.parent()
                .ok_or_else(|| format!("no parent for {}", to.display()))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            fs::copy(&from, &to)
                .map_err(|error| format!("failed to copy {}: {error}", from.display()))?;
        }
        let url = format!("../{GAMES_DIRECTORY}/{}/{WEB_GAME_MANIFEST_FILE}",
            encode_path_segment(&game_dir_name));
        let index = destination.join("index.html");
        fs::write(&index, template.replace("{{GAME_PATH}}", &url))
            .map_err(|error| format!("failed to write {}: {error}", index.display()))
    })();
    if let Err(error) = result {
        fs::remove_dir_all(destination).map_err(|cleanup| {
            format!("{error}; failed to clean up {}: {cleanup}", destination.display())
        })?;
        return Err(error);
    }
    Ok(WebSiteLayout { game_dir_name })
}

fn checked_game_directory(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.ends_with('.')
        || name.chars().any(|ch| ch.is_control() || "<>:\"/\\|?*".contains(ch))
    {
        return Err(format!("game name cannot be a portable directory name: {name:?}"));
    }
    let device = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3"
        | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2"
        | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9")
    {
        return Err(format!("game name is reserved on Windows: {name}"));
    }
    Ok(name.to_owned())
}

fn encode_path_segment(segment: &str) -> String {
    let mut encoded = String::new();
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn require_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("required player runtime directory {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!("not a player runtime directory: {}", path.display()));
    }
    Ok(())
}

fn require_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("required player runtime file {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("not a regular player runtime file: {}", path.display()));
    }
    Ok(())
}
