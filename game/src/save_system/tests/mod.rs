mod coordinator_tests;
mod document_tests;
mod latest_tests;
mod paths_tests;
mod registry_tests;

use engine_core::engine_global::set_game_name;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::sync::MutexGuard;

use crate::save_system::runtime_saves_root;

/// Acquires the global test lock and drops `runtime_saves_root()` on cleanup.
pub(super) struct CleanSaveRoot {
    _lock: MutexGuard<'static, ()>,
}

impl CleanSaveRoot {
    pub(super) fn new() -> Self {
        let _ = fs::remove_dir_all(runtime_saves_root());
        Self {
            _lock: game_fs_test_lock().lock().unwrap(),
        }
    }
}

impl Drop for CleanSaveRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(runtime_saves_root());
    }
}

pub(super) struct RuntimeSaveTestContext {
    _lock: MutexGuard<'static, ()>,
    game: TestGameFolder,
}

impl RuntimeSaveTestContext {
    pub(super) fn new(prefix: &str) -> Self {
        let lock = game_fs_test_lock().lock().unwrap();
        let game = TestGameFolder::new(prefix);
        set_game_name(game.name());
        let _ = fs::remove_dir_all(runtime_saves_root());
        Self { _lock: lock, game }
    }

    pub(super) fn game_name(&self) -> &str {
        self.game.name()
    }
}

impl Drop for RuntimeSaveTestContext {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(runtime_saves_root());
    }
}
