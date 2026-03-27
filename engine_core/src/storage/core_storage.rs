// engine_core/src/storage/core_storage.rs
use crate::constants::GAME_RON;
use crate::game::Game;
use crate::storage::path_utils::*;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Error;
use std::path::Path;
use uuid::Uuid;

pub type WorldIndex = HashMap<Uuid, String>;

/// Finds the game .ron in /Resources and returns an initialized `Game`.
pub async fn load_game_ron() -> io::Result<Game> {
    match resources_dir_from_exe() {
        Some(resources_folder) => match load_game_from_folder(&resources_folder).await {
            Ok(game) => Ok(game),
            Err(err) => Err(err),
        },
        None => Err(Error::other("Could not find resources folder")),
    }
}

/// Load the game .ron from a specified folder.
pub async fn load_game_from_folder(folder: &Path) -> io::Result<Game> {
    let path = folder.join(GAME_RON);
    let ron_string = fs::read_to_string(path)?;

    // Parse the RON
    match ron::from_str::<Game>(&ron_string) {
        Ok(game) => Ok(game),
        // Corrupt file
        Err(e) => Err(Error::other(e)),
    }
}
