use crate::save_system::{
    capture_document, runtime_latest_save_manifest_path, runtime_save_file, SaveCoordinatorError,
    SaveLane, SaveProviderRegistry, SaveSlotKey, LatestRuntimeSaveManifest, RonPersist,
    RUNTIME_SAVE_SCHEMA_VERSION, RuntimeSaveMetadata,
};
#[cfg(test)]
use crate::save_system::{SaveProvider, SaveProviderId};
use super::game_instance::GameInstance;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

/// The type of runtime load request, indicating which save to restore from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLoadRequest {
    /// Load the most recent save across any lane.
    Latest,
}

/// Orchestrates runtime save execution and pending load requests.
pub struct SaveRuntime {
    providers: Rc<RefCell<SaveProviderRegistry<'static>>>,
    pending_runtime_load_request: Option<RuntimeLoadRequest>,
}

impl SaveRuntime {
    pub fn new(providers: Rc<RefCell<SaveProviderRegistry<'static>>>) -> Self {
        Self {
            providers,
            pending_runtime_load_request: None,
        }
    }

    /// Captures state from all registered providers and persists to `lane`.
    pub fn save_to_lane(
        &mut self,
        game_instance: &Rc<RefCell<GameInstance>>,
        lane: SaveLane,
    ) -> io::Result<()> {
        let game = &game_instance.borrow().game;
        let metadata = RuntimeSaveMetadata {
            schema_version: RUNTIME_SAVE_SCHEMA_VERSION,
            game_id: game.id,
            game_name: game.name.clone(),
            lane,
            slot: SaveSlotKey::Default,
            saved_at_unix_ms: current_unix_ms(),
        };

        let document = capture_document(&mut self.providers.borrow_mut(), &metadata)
            .map_err(coordinator_to_io_error)?;
        let path = runtime_save_file(&metadata.slot, lane);
        document.write_to_path(&path)?;
        LatestRuntimeSaveManifest {
            lane,
            slot: metadata.slot.clone(),
            saved_at_unix_ms: metadata.saved_at_unix_ms,
        }
        .write_to_path(&runtime_latest_save_manifest_path())?;
        Ok(())
    }

    /// Records a pending request to load the latest runtime save.
    pub fn request_latest_runtime_load(&mut self) {
        self.pending_runtime_load_request = Some(RuntimeLoadRequest::Latest);
    }

    /// Returns the pending load request, if any.
    pub fn pending_runtime_load_request(&self) -> Option<RuntimeLoadRequest> {
        self.pending_runtime_load_request
    }
}

fn current_unix_ms() -> u64 {
    // Clock-before-epoch is unreachable on modern hardware.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn coordinator_to_io_error(error: SaveCoordinatorError) -> io::Error {
    match error {
        SaveCoordinatorError::Capture { source, .. }
        | SaveCoordinatorError::Apply { source, .. } => source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_system::{RestorePhase, SavedSection};
    use engine_core::engine_global::set_game_name;
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use engine_core::game::Game;
    use std::collections::HashMap;

    struct StubSaveProvider;

    impl SaveProvider for StubSaveProvider {
        fn id(&self) -> SaveProviderId {
            SaveProviderId::new("game.flags")
        }

        fn restore_phase(&self) -> RestorePhase {
            RestorePhase::PostRuntime
        }

        fn capture(&mut self) -> io::Result<SavedSection> {
            Ok(SavedSection {
                version: 1,
                data: String::from("room=2"),
            })
        }

        fn apply(&mut self, _section: &SavedSection) -> io::Result<()> {
            Ok(())
        }
    }

    fn setup_save_runtime(
        prefix: &str,
    ) -> (
        TestGameFolder,
        Rc<RefCell<GameInstance>>,
        SaveRuntime,
    ) {
        let folder = TestGameFolder::new(prefix);
        set_game_name(folder.name());

        let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
        save_providers
            .borrow_mut()
            .register(Box::new(StubSaveProvider))
            .unwrap();

        let mut game = Game::with_name(folder.name());
        let game_instance = Rc::new(RefCell::new(GameInstance {
            game,
            prev_positions: HashMap::new(),
        }));

        (folder, game_instance, SaveRuntime::new(save_providers))
    }

    #[test]
    fn save_to_lane_writes_document_and_latest_manifest() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let (_folder, game_instance, mut save_runtime) =
            setup_save_runtime("save_runtime_manual");

        save_runtime
            .save_to_lane(&game_instance, SaveLane::Manual)
            .unwrap();

        let save_path = runtime_save_file(&SaveSlotKey::Default, SaveLane::Manual);
        let manifest_path = runtime_latest_save_manifest_path();
        assert!(save_path.exists());
        assert!(manifest_path.exists());
    }

    #[test]
    fn request_latest_runtime_load_sets_pending_request() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let (_folder, _game_instance, mut save_runtime) =
            setup_save_runtime("save_runtime_latest");

        save_runtime.request_latest_runtime_load();

        assert_eq!(
            save_runtime.pending_runtime_load_request(),
            Some(RuntimeLoadRequest::Latest)
        );
    }
}
