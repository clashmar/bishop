use crate::storage::path_utils::game_folder;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

/// Returns the shared filesystem lock for tests that mutate game folders.
pub fn game_fs_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Owns a temporary game folder under `games/` for the duration of a test.
pub struct TestGameFolder {
    name: String,
}

impl TestGameFolder {
    /// Creates a unique temporary game folder name and removes any stale folder at that path.
    pub fn new(prefix: &str) -> Self {
        let name = format!("{prefix}_{}", Uuid::new_v4());
        let path = game_folder(&name);
        let _ = fs::remove_dir_all(&path);
        Self { name }
    }

    /// Returns the game name used for this temporary folder.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the absolute path for this temporary game folder.
    pub fn path(&self) -> PathBuf {
        game_folder(&self.name)
    }
}

impl Drop for TestGameFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.path());
    }
}
