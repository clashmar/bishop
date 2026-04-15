use engine_core::storage::editor_config::app_dir;
use engine_core::storage::path_utils::absolute_save_root;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;

/// Return the name of the most recently modified game folder.
pub fn most_recent_game_name() -> Option<String> {
    let root = absolute_save_root();
    let mut best: Option<(String, std::time::SystemTime)> = None;

    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Ok(mod_time) = entry.metadata().ok()?.modified() {
            match best {
                None => best = Some((name, mod_time)),
                Some((_, t)) if mod_time > t => best = Some((name, mod_time)),
                _ => {}
            }
        }
    }
    best.map(|(name, _)| name)
}

/// Writes an embedded slice of bytes to the system app directory and returns the path or error.
pub fn write_to_app_dir(filename: &str, embedded: &[u8]) -> io::Result<PathBuf> {
    let mut path = app_dir();
    fs::create_dir_all(&path)?;

    path.push(filename);

    let mut file = fs::File::create(&path)?;
    file.write_all(embedded)?;

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)?;
    }

    Ok(path)
}
