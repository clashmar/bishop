use crate::storage::export::{export_game, export_target_path};
use engine_core::game::Game;
use engine_core::storage::web_site_export::stage_web_site;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportTarget {
    Web,
    NativeHost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePlatform {
    Mac,
    Windows,
    Linux,
}

pub fn host_platform() -> NativePlatform {
    if cfg!(target_os = "macos") {
        NativePlatform::Mac
    } else if cfg!(target_os = "windows") {
        NativePlatform::Windows
    } else {
        NativePlatform::Linux
    }
}

pub fn supported_native_target(platform: NativePlatform, host: NativePlatform) -> Option<ExportTarget> {
    match (platform, host) {
        (NativePlatform::Mac, NativePlatform::Mac)
        | (NativePlatform::Windows, NativePlatform::Windows) => Some(ExportTarget::NativeHost),
        _ => None,
    }
}

pub struct PendingExport {
    pub dest_root: PathBuf,
    pub target: ExportTarget,
}

pub fn export_target_path_for(dest_root: &Path, game: &Game, target: ExportTarget) -> PathBuf {
    match target {
        ExportTarget::Web => dest_root.join("Bishop Web Export"),
        ExportTarget::NativeHost => export_target_path(dest_root, game),
    }
}

/// Stage the chosen package before replacing an approved existing export. Files outside the
/// selected package are untouched, and failures before the final swap leave the old package intact.
pub fn export_game_for_target(
    dest_root: &Path,
    resources: &Path,
    runtime: &Path,
    game: &Game,
    target: ExportTarget,
    overwrite: bool,
) -> io::Result<PathBuf> {
    let destination = export_target_path_for(dest_root, game, target);
    let previous = match fs::symlink_metadata(&destination) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if let Some(metadata) = &previous {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(io::Error::other(format!(
                "export destination is not a regular directory: {}", destination.display()
            )));
        }
        if !overwrite {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists,
                format!("export destination already exists: {}", destination.display())));
        }
    }

    let staging_root = dest_root.join(format!(".bishop-export-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging_root)?;
    let _cleanup = StagingGuard(staging_root.clone());
    let staged_package = export_target_path_for(&staging_root, game, target);
    let staged_output = match target {
        ExportTarget::Web => {
            stage_web_site(resources, runtime, &staged_package).map_err(io::Error::other)?;
            staged_package.clone()
        }
        ExportTarget::NativeHost => export_game(&staging_root, game)?,
    };
    let output = match staged_output.strip_prefix(&staged_package) {
        Ok(relative) if !relative.as_os_str().is_empty() => destination.join(relative),
        _ => destination.clone(),
    };

    if previous.is_some() {
        let backup = dest_root.join(format!(".bishop-previous-{}", uuid::Uuid::new_v4()));
        if fs::symlink_metadata(&backup).is_ok() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists,
                format!("backup destination already exists: {}", backup.display())));
        }
        fs::rename(&destination, &backup)?;
        if let Err(error) = fs::rename(&staged_package, &destination) {
            fs::rename(&backup, &destination).map_err(|rollback| io::Error::other(
                format!("{error}; could not restore previous export: {rollback}")
            ))?;
            return Err(error);
        }
        fs::remove_dir_all(&backup)?;
    } else {
        fs::rename(&staged_package, &destination)?;
    }
    Ok(output)
}

struct StagingGuard(PathBuf);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::storage::save_game_to_folder;
    use engine_core::worlds::{Room, RoomId, World, WorldId};
    use std::fs;
    use tempfile::TempDir;

    fn fixture(root: &Path, runtime: &Path) -> Game {
        let mut game = Game::with_name("Second Game");
        let mut world = World::new(WorldId(2), "Second World".into(), 16.0);
        world.current_room_id = Some(RoomId(3));
        world.add_room(Room::new(&mut game.ecs, RoomId(3), 16.0));
        game.add_world(world);
        game.current_world_id = Some(WorldId(2));
        save_game_to_folder(&game, root).unwrap();
        fs::create_dir_all(runtime.join("player")).unwrap();
        for name in ["player.html", "input.js", "audio.js", "web_shell.js",
                     "web_shell.wasm", "player/config.mjs"] {
            fs::write(runtime.join(name), name).unwrap();
        }
        fs::write(runtime.join("index-template.html"), "<iframe src='{{GAME_PATH}}'></iframe>").unwrap();
        game
    }

    #[test]
    fn web_target_uses_shared_site_packager_without_replacing_existing_destination() {
        let resources = TempDir::new().unwrap();
        let runtime = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let game = fixture(resources.path(), runtime.path());
        let target = export_target_path_for(output.path(), &game, ExportTarget::Web);
        assert_eq!(target, output.path().join("Bishop Web Export"));

        let path = export_game_for_target(output.path(), resources.path(), runtime.path(), &game,
                                          ExportTarget::Web, false).unwrap();
        assert_eq!(path, target);
        assert_eq!(fs::read(target.join("games/Second Game/game.ron")).unwrap(),
                   fs::read(resources.path().join("game.ron")).unwrap());
        assert_eq!(fs::read_to_string(target.join("bishop-player/web_shell.js")).unwrap(),
                   "web_shell.js");

        fs::write(target.join("keep.txt"), "prior data").unwrap();
        assert!(export_game_for_target(output.path(), resources.path(), runtime.path(), &game,
                                       ExportTarget::Web, false).is_err());
        assert_eq!(fs::read_to_string(target.join("keep.txt")).unwrap(), "prior data");
    }

    #[test]
    fn confirmed_web_overwrite_replaces_only_target_after_successful_staging() {
        let resources = TempDir::new().unwrap();
        let runtime = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let game = fixture(resources.path(), runtime.path());
        let target = export_target_path_for(output.path(), &game, ExportTarget::Web);
        fs::create_dir(&target).unwrap();
        fs::write(target.join("old.txt"), "previous").unwrap();
        fs::write(output.path().join("unrelated.txt"), "keep").unwrap();
        let missing_runtime = TempDir::new().unwrap();
        assert!(export_game_for_target(output.path(), resources.path(), missing_runtime.path(), &game,
                                       ExportTarget::Web, true).is_err());
        assert_eq!(fs::read_to_string(target.join("old.txt")).unwrap(), "previous");

        export_game_for_target(output.path(), resources.path(), runtime.path(), &game,
                               ExportTarget::Web, true).unwrap();
        assert!(!target.join("old.txt").exists());
        assert!(target.join("index.html").is_file());
        assert_eq!(fs::read_to_string(output.path().join("unrelated.txt")).unwrap(), "keep");
    }

    #[test]
    fn unsupported_native_platforms_are_not_selectable() {
        assert_eq!(supported_native_target(NativePlatform::Mac, NativePlatform::Mac),
                   Some(ExportTarget::NativeHost));
        assert_eq!(supported_native_target(NativePlatform::Windows, NativePlatform::Mac), None);
        assert_eq!(supported_native_target(NativePlatform::Linux, NativePlatform::Linux), None);
        assert_eq!(supported_native_target(NativePlatform::Windows, NativePlatform::Windows),
                   Some(ExportTarget::NativeHost));
    }

    #[test]
    fn native_host_retains_previous_target_naming() {
        let game = Game::with_name("My Game");
        let root = Path::new("/tmp/exports");
        assert_eq!(export_target_path_for(root, &game, ExportTarget::NativeHost),
                   export_target_path(root, &game));
    }
}
