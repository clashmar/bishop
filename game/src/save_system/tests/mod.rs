mod document_tests;
mod latest_tests;
mod paths_tests;

use engine_core::engine_global::set_game_name;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::sync::MutexGuard;

use crate::save_system::runtime_saves_root;

/// Drops `runtime_saves_root()` on cleanup (no game-folder setup).
pub(super) struct CleanSaveRoot;

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
