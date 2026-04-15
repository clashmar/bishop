#[cfg(all(target_os = "macos", not(debug_assertions)))]
use crate::playtest_binaries::PLAYTEST_BIN;
#[cfg(all(target_os = "windows", not(debug_assertions)))]
use crate::playtest_binaries::PLAYTEST_EXE;
#[cfg(not(debug_assertions))]
use crate::storage_shared::write_to_app_dir;
use engine_core::prelude::*;
use engine_core::storage::editor_config::get_startup_mode;
use engine_core::storage::path_utils::resources_folder;
use std::io;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

/// Serialise everything the playtest binary needs and return the temporary payload path.
pub fn write_playtest_payload(room: &Room, game: &Game) -> io::Result<PathBuf> {
    let game_ron = ron::to_string(game)
        .map_err(|error| io::Error::other(format!("Could not serialize game: {error}")))?;

    let mut game_copy: Game = ron::from_str(&game_ron)
        .map_err(|error| io::Error::other(format!("Could not deserialize game: {error}")))?;

    game_copy.ecs.set_player_spawn_from_proxy(room.id);
    game_copy.ecs.purge_proxies();
    let startup_path = resources_folder(&game.name).join("startup.ron");
    let startup_ron = fs::read_to_string(&startup_path).ok();

    #[derive(serde::Serialize)]
    struct Payload<'a> {
        room: &'a Room,
        game: &'a Game,
        startup_ron: Option<String>,
        startup_mode: StartupMode,
    }

    let payload = Payload {
        room,
        game: &game_copy,
        startup_ron,
        startup_mode: get_startup_mode(),
    };

    let ron = ron::ser::to_string_pretty(&payload, ron::ser::PrettyConfig::default())
        .map_err(|error| io::Error::other(format!("Could not serialize payload: {error}")))?;

    let mut temp_path = env::temp_dir();
    temp_path.push(format!("playtest_{}.ron", uuid::Uuid::new_v4()));
    fs::write(&temp_path, ron)?;
    Ok(temp_path)
}

/// Return the absolute path to the playtest executable, building or unpacking it when needed.
pub fn resolve_playtest_binary() -> io::Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let exe_name = "game-playtest.exe";
    #[cfg(target_os = "macos")]
    let exe_name = "game-playtest";

    #[cfg(not(debug_assertions))]
    {
        #[cfg(target_os = "windows")]
        {
            return write_to_app_dir(exe_name, PLAYTEST_EXE);
        }
        #[cfg(target_os = "macos")]
        {
            return write_to_app_dir(exe_name, PLAYTEST_BIN);
        }
    }

    let mut exe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    exe_path.pop();
    exe_path.push("target");
    exe_path.push("release");
    exe_path.push(exe_name);

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("game")
        .arg("--bin")
        .arg("game-playtest")
        .arg("--release");

    let status = cmd.status()?;

    if status.success() {
        Ok(exe_path)
    } else {
        Err(Error::other("Playtest build failed."))
    }
}
