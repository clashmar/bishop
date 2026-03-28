// editor\src\storage\export.rs
#![allow(unused)]
use crate::editor_assets::assets::*;
use crate::storage::sound_preset_storage::SOUND_PRESETS_RON;
use engine_core::storage::path_utils::*;
use engine_core::constants::*;
use engine_core::game::*;
use bishop::prelude::*;
use engine_core::*;
use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
pub async fn export_game(game: &Game) -> io::Result<PathBuf> {
    let dest_root = rfd::FileDialog::new()
        .set_title("Select destination folder for export:")
        .pick_folder()
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "No destination folder was selected.",
            )
        })?;

    // TODO: This overwrites, check for duplicates

    #[cfg(target_os = "windows")]
    {
        onscreen_info!("Exporting for windows");
        let exe_path = export_for_windows(&dest_root, game).await?;
        Ok(exe_path)
    }
    #[cfg(target_os = "macos")]
    {
        let bundle_path = export_for_mac(dest_root, game).await?;
        Ok(bundle_path)
    }

    // TODO Handle Linux
}

#[cfg(windows)]
async fn export_for_windows(dest_root: &PathBuf, game: &Game) -> io::Result<PathBuf> {
    let target_package = dest_root.join(format!("{}", &game.name));

    // Guard will clear up the package if there is an error
    let mut guard = ExportGuard::new(target_package.clone());

    // Write the .exe to the package directory
    fs::create_dir_all(&target_package)?;
    let exe_path = &target_package.join(format!("{}.exe", game.name));

    onscreen_debug!("Creating new .exe at: {}", exe_path.display());
    let mut exe_file = fs::File::create(&exe_path)?;

    onscreen_debug!("Writing buffer into .exe");
    exe_file.write_all(GAME_EXE)?;

    onscreen_debug!("Updating .exe");
    if let Err(e) = update_exe(&exe_path, game) {
        return Err(Error::other(format!("Could not update .exe: {e}")));
    }

    // Everything else goes in /Resources to mirror macOS structure
    // Skip source files that aren't needed for the final game
    let src_resources = resources_folder_current();
    let target_resources = target_package.join(RESOURCES_FOLDER);
    let skip_extensions = &["json", "aseprite", "ase"];
    copy_dir_filtered(&src_resources, &target_resources, skip_extensions)?;
    let _ = fs::remove_file(target_resources.join(SOUND_PRESETS_RON));

    // Overwrite game.ron purging player proxies
    let game_ron = ron::to_string(game).map_err(|e| io::Error::other(e.to_string()))?;
    let mut game_copy: Game =
        ron::from_str(&game_ron).map_err(|e| io::Error::other(e.to_string()))?;

    // Set player spawn position from proxy before purging
    if let Some(start_room_id) = game_copy.current_world().starting_room_id {
        game_copy.ecs.set_player_spawn_from_proxy(start_room_id);
    }
    game_copy.ecs.purge_proxies();

    let ron_string = ron::to_string(&game_copy).map_err(|e| io::Error::other(e.to_string()))?;
    fs::write(target_resources.join(GAME_RON), ron_string)?;

    // TODO: Write manifest for game

    // Tell the guard to keep the folder
    guard.success();
    Ok(target_package)
}

#[cfg(unix)]
async fn export_for_mac(dest_root: PathBuf, game: &Game) -> io::Result<PathBuf> {
    let bundle_path = dest_root.join(format!("{}.app", game.name));

    // Guard will clear up the export if there are errors
    let mut guard = ExportGuard::new(bundle_path.clone());

    // Write the game binary to the bundle
    let macos_dir = bundle_path.join(CONTENTS_FOLDER).join("MacOS");

    // Make sure this file exists
    fs::create_dir_all(&macos_dir)?;

    let bin_path = &macos_dir.join(game.name.clone());

    onscreen_debug!("Creating new binary at: {}", bin_path.display());
    let mut bin_file = fs::File::create(bin_path)?;

    onscreen_debug!("Writing buffer into binary.");
    bin_file.write_all(GAME_BIN)?;
    bin_file.flush()?;

    // Set executable permissions
    onscreen_debug!("Writing binary permissions.");
    let mut permissions = fs::metadata(bin_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(bin_path, permissions)?;

    // Copy /Resources, skipping source files not needed for the final game
    onscreen_debug!("Copying /Resources.");
    let src_resources = resources_folder_current();
    let target_resources = bundle_path.join(CONTENTS_FOLDER).join(RESOURCES_FOLDER);

    let skip_extensions = &["json", "aseprite", "ase"];
    copy_dir_filtered(&src_resources, &target_resources, skip_extensions)?;
    let _ = fs::remove_file(target_resources.join(SOUND_PRESETS_RON));

    // Overwrite game.ron purging player proxies
    let game_ron = ron::to_string(game).map_err(|e| io::Error::other(e.to_string()))?;
    let mut game_copy: Game =
        ron::from_str(&game_ron).map_err(|e| io::Error::other(e.to_string()))?;

    // Set player spawn position from proxy before purging
    if let Some(start_room_id) = game_copy.current_world().starting_room_id {
        game_copy.ecs.set_player_spawn_from_proxy(start_room_id);
    }
    game_copy.ecs.purge_proxies();

    let ron_string = ron::to_string(&game_copy).map_err(|e| io::Error::other(e.to_string()))?;
    fs::write(target_resources.join(GAME_RON), ron_string)?;

    // Copy Icon.icns
    onscreen_debug!("Copying Icon.icns.");
    let src_icns = mac_os_folder().join("Icon.icns");
    let target_icns = target_resources.join("Icon.icns");

    if src_icns.exists() {
        fs::copy(&src_icns, &target_icns)?;
    } else {
        onscreen_debug!("Icon.icns not found, skipping.");
    }

    // Copy Info.plist
    if let Some(bundle_assets) = bundle_assets_folder() {
        // TODO: add more to plist
        onscreen_debug!("Copying Info.plist.");
        let src_plist = bundle_assets.join("Info.plist");
        let target_plist = bundle_path.join(CONTENTS_FOLDER).join("Info.plist");
        let _ = fs::copy(src_plist, target_plist);
    }

    // Tell the guard to keep the folder
    guard.success();
    onscreen_debug!("Export successful.");
    Ok(bundle_path)
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
        onscreen_debug!("Replacing .ico from: {}", icon_path.display());
        if let Some(icon_resource) = resources.find(resource_type::ICON, Id::Integer(1)) {
            icon_resource.replace(&png_bytes)?.update()?;
        }
    } else {
        onscreen_warn!("Could not read .ico");
    }

    if let Some(mut version_info) = resources.get_version_info()? {
        onscreen_debug!("Updating version info");
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
        onscreen_warn!("Could not get version info");
    }

    Ok(())
}
