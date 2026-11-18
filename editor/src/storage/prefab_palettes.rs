use crate::prefab::palette::PrefabPaletteState;
use crate::storage::game_io::editor_metadata_path;
use engine_core::storage::editor_metadata_folder;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};

const PREFAB_PALETTE_RON: &str = "prefab_palette.ron";

/// Saves the room prefab palette state for the game.
pub fn save_prefab_palette_state(
    game_name: &str,
    state: &PrefabPaletteState,
) -> io::Result<()> {
    let dir = editor_metadata_folder(game_name);
    fs::create_dir_all(&dir)?;
    let path = dir.join(PREFAB_PALETTE_RON);
    let ron =
        ron::ser::to_string_pretty(state, ron::ser::PrettyConfig::new()).map_err(Error::other)?;
    fs::write(path, ron)
}

/// Loads the room prefab palette state for the game.
pub fn load_prefab_palette_state(game_name: &str) -> io::Result<PrefabPaletteState> {
    let path = editor_metadata_path(game_name, PREFAB_PALETTE_RON);

    match fs::read_to_string(&path) {
        Ok(ron) => ron::from_str(&ron).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Could not parse prefab palette state: {error}"),
            )
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(PrefabPaletteState::default()),
        Err(error) => Err(error),
    }
}
