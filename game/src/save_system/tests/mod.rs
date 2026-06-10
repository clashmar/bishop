mod coordinator_tests;
mod document_tests;
mod latest_tests;
mod lua_provider_tests;
mod paths_tests;
mod registry_tests;

use engine_core::engine_global::set_game_name;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::sync::MutexGuard;

use crate::save_system::runtime_saves_root;

pub(crate) fn cleanup_runtime_saves() {
    let runtime_saves = runtime_saves_root();
    let runtime_saves_root = runtime_saves.parent().map(|path| path.to_path_buf());
    let _ = fs::remove_dir_all(&runtime_saves);
    if let Some(root) = runtime_saves_root {
        if root.exists()
            && root
                .read_dir()
                .is_ok_and(|mut entries| entries.next().is_none())
        {
            let _ = fs::remove_dir_all(root);
        }
    }
}

/// Acquires the global test lock and drops `runtime_saves_root()` on cleanup.
pub(crate) struct CleanSaveRoot {
    _lock: MutexGuard<'static, ()>,
}

impl CleanSaveRoot {
    pub(crate) fn new() -> Self {
        let lock = game_fs_test_lock().lock().unwrap();
        set_game_name("clean_save_root");
        cleanup_runtime_saves();
        Self { _lock: lock }
    }
}

impl Drop for CleanSaveRoot {
    fn drop(&mut self) {
        cleanup_runtime_saves();
    }
}

pub(crate) struct RuntimeSaveTestContext {
    _lock: MutexGuard<'static, ()>,
    game: TestGameFolder,
}

impl RuntimeSaveTestContext {
    pub(crate) fn new(prefix: &str) -> Self {
        let lock = game_fs_test_lock().lock().unwrap();
        let game = TestGameFolder::new(prefix);
        set_game_name(game.name());
        cleanup_runtime_saves();
        Self { _lock: lock, game }
    }

    pub(crate) fn game_name(&self) -> &str {
        self.game.name()
    }
}

impl Drop for RuntimeSaveTestContext {
    fn drop(&mut self) {
        cleanup_runtime_saves();
    }
}
