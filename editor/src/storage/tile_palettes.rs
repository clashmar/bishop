use crate::storage::game_io::editor_metadata_path;
use crate::tilemap::tile_palette::TilePalette;
use engine_core::storage::editor_metadata_folder;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};

const TILE_PALETTE_RON: &str = "palette.ron";

/// Save the palette for the game.
pub fn save_palette(palette: &TilePalette, game_name: &str) -> io::Result<()> {
    let dir = editor_metadata_folder(game_name);
    fs::create_dir_all(&dir)?;
    let path = dir.join(TILE_PALETTE_RON);
    let ron = ron::ser::to_string(palette).map_err(Error::other)?;
    fs::write(path, ron)
}

/// Load the palette from the game folder.
pub fn load_palette(game_name: &str) -> io::Result<TilePalette> {
    let path = editor_metadata_path(game_name, TILE_PALETTE_RON);
    match fs::read_to_string(path) {
        Ok(ron) => ron::de::from_str(&ron).map_err(Error::other),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(TilePalette::new()),
        Err(error) => Err(error),
    }
}
