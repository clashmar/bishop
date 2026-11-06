#![allow(unused)]
use crate::editor_assets::assets::*;
use bishop::prelude::*;
use engine_core::constants::{paths, ui, world};
use engine_core::ecs::*;
use engine_core::engine_global::{game_name};
use engine_core::game::{Game};
use engine_core::logging::{omni_debug, omni_info, omni_warn};
use engine_core::storage::*;
use ::widgets::*;
use engine_core::worlds::*;
use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use winres_edit::resource_type;
use winres_edit::Id;
use winres_edit::Resources;

/// Removes `path` when dropped unless `success()` has been called.
struct ExportGuard {
    path: PathBuf,
    ok: bool,
}

impl ExportGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, ok: false }
    }

    fn success(&mut self) {
        self.ok = true;
    }
}

impl Drop for ExportGuard {
    fn drop(&mut self) {
        if !self.ok && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Exports the game to the chosen folder on all platforms.
pub fn export_game(dest_root: &Path, game: &Game) -> io::Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        omni_info!("Exporting for windows");
        let exe_path = export_for_windows(dest_root, game)?;
        Ok(exe_path)
    }
    #[cfg(target_os = "macos")]
    {
        let bundle_path = export_for_mac(dest_root, game)?;
        Ok(bundle_path)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    Err(io::Error::new(ErrorKind::Unsupported, "Native export is not supported on this host"))
}

/// Returns the package/bundle path that exporting will overwrite.
pub fn export_target_path(dest_root: &Path, game: &Game) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return dest_root.join(&game.name);
    }

    #[cfg(target_os = "macos")]
    {
        return dest_root.join(format!("{}.app", game.name));
    }

    #[allow(unreachable_code)]
    dest_root.join(&game.name)
}

fn export_game_copy(game: &Game) -> io::Result<Game> {
    let game_ron = ron::to_string(game).map_err(|e| io::Error::other(e.to_string()))?;
    let mut game_copy: Game =
        ron::from_str(&game_ron).map_err(|e| io::Error::other(e.to_string()))?;

    game_copy.rebuild_world_index();
    for world in game_copy.worlds_mut() {
        world.rebuild_room_grid();
    }

    Ok(game_copy)
}

fn write_exported_game(game: &Game, target_resources: &Path) -> io::Result<()> {
    let game_copy = export_game_copy(game)?;
    save_game_to_folder(&game_copy, target_resources)
}

#[cfg(windows)]
fn export_for_windows(dest_root: &Path, game: &Game) -> io::Result<PathBuf> {
    let target_package = export_target_path(dest_root, game);

    // Guard will clear up the package if there is an error
    let mut guard = ExportGuard::new(target_package.clone());

    // Write the .exe to the package directory
    fs::create_dir_all(&target_package)?;
    let exe_path = &target_package.join(format!("{}.exe", game.name));

    omni_debug!("Creating new .exe at: {}", exe_path.display());
    let mut exe_file = fs::File::create(&exe_path)?;

    omni_debug!("Writing buffer into .exe");
    exe_file.write_all(GAME_EXE)?;

    omni_debug!("Updating .exe");
    if let Err(e) = update_exe(&exe_path, game) {
        return Err(Error::other(format!("Could not update .exe: {e}")));
    }

    // Everything else goes in /Resources to mirror macOS structure
    // Skip source files that aren't needed for the final game
    let src_resources = resources_folder_current();
    let target_resources = target_package.join(paths::RESOURCES_FOLDER);
    let skip_extensions = &["json", "aseprite", "ase"];
    copy_dir_filtered(&src_resources, &target_resources, skip_extensions)?;

    write_exported_game(game, &target_resources)?;

    // Tell the guard to keep the folder
    guard.success();
    Ok(target_package)
}

#[cfg(unix)]
fn export_for_mac(dest_root: &Path, game: &Game) -> io::Result<PathBuf> {
    let bundle_path = export_target_path(dest_root, game);

    // Guard will clear up the export if there are errors
    let mut guard = ExportGuard::new(bundle_path.clone());

    // Write the game binary to the bundle
    let macos_dir = bundle_path.join(paths::CONTENTS_FOLDER).join("MacOS");

    // Make sure this file exists
    fs::create_dir_all(&macos_dir)?;

    let bin_path = &macos_dir.join(game.name.clone());

    omni_debug!("Creating new binary at: {}", bin_path.display());
    let mut bin_file = fs::File::create(bin_path)?;

    omni_debug!("Writing buffer into binary.");
    bin_file.write_all(GAME_BIN)?;
    bin_file.flush()?;

    // Set executable permissions
    omni_debug!("Writing binary permissions.");
    let mut permissions = fs::metadata(bin_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(bin_path, permissions)?;

    // Copy /Resources, skipping source files not needed for the final game
    omni_debug!("Copying /Resources.");
    let src_resources = resources_folder_current();
    let target_resources = bundle_path
        .join(paths::CONTENTS_FOLDER)
        .join(paths::RESOURCES_FOLDER);

    let skip_extensions = &["json", "aseprite", "ase"];
    copy_dir_filtered(&src_resources, &target_resources, skip_extensions)?;

    write_exported_game(game, &target_resources)?;

    // Copy Icon.icns
    omni_debug!("Copying Icon.icns.");
    let src_icns = mac_os_folder().join("Icon.icns");
    let target_icns = target_resources.join("Icon.icns");

    if src_icns.exists() {
        fs::copy(&src_icns, &target_icns)?;
    } else {
        omni_debug!("Icon.icns not found, skipping.");
    }

    let target_plist = bundle_path.join(paths::CONTENTS_FOLDER).join("Info.plist");
    omni_debug!("Writing Info.plist.");
    fs::write(&target_plist, mac_export_info_plist(game))?;

    // Tell the guard to keep the folder
    guard.success();
    omni_debug!("Export successful.");
    Ok(bundle_path)
}

#[cfg(unix)]
fn mac_export_info_plist(game: &Game) -> String {
    let identifier = format!(
        "com.clashmar.{}",
        game.name
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    );

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIconFile</key>
    <string>Icon</string>
    <key>CFBundleIdentifier</key>
    <string>{identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
</dict>
</plist>
"#,
        game.name, game.name
    )
}

/// Updates the game .exe with the game information.
#[cfg(windows)]
fn update_exe(exe_path: &PathBuf, game: &Game) -> Result<(), winres_edit::Error> {
    let resources = Resources::new(&exe_path);

    let icon_path = windows_folder().join("Icon.ico");

    // TODO: Maybe 1 PNG which the program can handle
    // all together using Image or .ico crate and icns

    // Read the file and replace the icon
    if let Ok(png_bytes) = fs::read(&icon_path) {
        omni_debug!("Replacing .ico from: {}", icon_path.display());
        if let Some(icon_resource) = resources.find(resource_type::ICON, Id::Integer(1)) {
            icon_resource.replace(&png_bytes)?.update()?;
        }
    } else {
        omni_warn!("Could not read .ico");
    }

    if let Some(mut version_info) = resources.get_version_info()? {
        omni_debug!("Updating version info");
        // TODO: Update with actual version
        let version: [u16; 4] = [0, 1, 0, 0];

        let game_name = game.name.as_str();

        version_info
            .set_file_version(&version)
            .set_product_version(&version)
            .insert_strings(&[
                // TODO: Use real values:
                ("ProductName", game_name),
                ("OriginalFilename", format!("{}.exe", game_name).as_str()),
                ("FileDescription", "Game Description."),
                ("LegalCopyright", "© 2025 Clashmar"),
                ("LegalTrademark", "Bishop Engine™"),
                ("CompanyName", "Clashmar Ltd."),
                ("Comments", "A 2D Game made with Bishop Engine"),
                ("InternalName", game_name),
            ])
            .update()?;
    } else {
        omni_warn!("Could not get version info");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn native_export_keeps_start_room_proxy_for_runtime_spawn() {
        let folder = tempfile::tempdir().unwrap();
        let room_id = RoomId(1);
        let mut game = Game::with_name("ExportFixture");
        let mut world = World::new(WorldId(1), "Start".to_string(), 8.0);
        world.add_room(Room { id: room_id, ..Default::default() });
        game.add_world(world);
        let mut other_world = World::new(WorldId(2), "Other".to_string(), 8.0);
        other_world.add_room(Room { id: RoomId(2), ..Default::default() });
        game.add_world(other_world);
        let entry_position = Vec2::new(20.0, 16.0);
        let entry = game.ecs.create_entity()
            .with(WorldEntry { name: WorldEntry::START_NAME.to_string(), is_start: true })
            .with(Transform { position: entry_position, ..Default::default() })
            .with_current_room(room_id).finish();
        let player_position = Vec2::new(5.0, 6.0);
        let player = game.ecs.create_entity().with(Player)
            .with(Transform { position: player_position, ..Default::default() }).finish();
        let proxy_position = Vec2::new(28.0, 72.0);
        let proxy = game.ecs.create_entity().with(PlayerProxy)
            .with(Transform { position: proxy_position, ..Default::default() })
            .with_current_room(room_id).finish();

        write_exported_game(&game, folder.path()).unwrap();
        let loaded = load_full_game_from_folder(folder.path()).unwrap();

        assert_eq!(loaded.start_world_id, Some(WorldId(1)));
        assert_eq!(loaded.current_world_id, Some(WorldId(2)));
        assert!(loaded.get_world(WorldId(1)).is_some());
        assert_eq!(loaded.ecs.get_player_proxy(room_id), Some(proxy));
        assert_eq!(loaded.ecs.get::<Transform>(proxy).map(|transform| transform.position), Some(proxy_position));
        assert_eq!(loaded.ecs.get::<Transform>(entry).unwrap().position, entry_position);
        assert!(loaded.ecs.get::<WorldEntry>(entry).unwrap().is_start);
        assert_eq!(loaded.ecs.get::<Transform>(player).unwrap().position, player_position);
        assert!(loaded.ecs.get::<CurrentRoom>(player).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn export_target_path_uses_app_bundle_name_on_macos() {
        let mut game = Game::default();
        game.name = "Demo".to_string();
        let path = export_target_path(
            Path::new("/tmp/exports"),
            &game,
        );

        assert_eq!(path, PathBuf::from("/tmp/exports/Demo.app"));
    }

    #[cfg(windows)]
    #[test]
    fn export_target_path_uses_plain_folder_name_on_windows() {
        let mut g = Game::default();
        g.name = "Demo".to_string();
        let path = export_target_path(
            Path::new(r"C:\exports"),
            &g,
        );

        assert_eq!(path, PathBuf::from(r"C:\exports\Demo"));
    }

    #[cfg(unix)]
    #[test]
    fn mac_export_info_plist_contains_required_bundle_keys() {
        let mut game = Game::default();
        game.name = "Demo".to_string();
        let plist = mac_export_info_plist(&game);

        assert!(plist.contains("<key>CFBundleExecutable</key>"));
        assert!(plist.contains("<string>Demo</string>"));
        assert!(plist.contains("<key>CFBundlePackageType</key>"));
        assert!(plist.contains("<string>APPL</string>"));
        assert!(plist.contains("<key>CFBundleIconFile</key>"));
        assert!(plist.contains("<string>Icon</string>"));
    }

    #[test]
    fn export_guard_removes_failed_output_on_drop() {
        let path =
            std::env::temp_dir().join(format!("bishop-export-guard-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        {
            let _guard = ExportGuard::new(path.clone());
            assert!(path.exists());
        }

        assert!(!path.exists());
    }
}
