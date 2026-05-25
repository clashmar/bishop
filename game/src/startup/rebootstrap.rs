//! Startup request model and runtime-triggered rebootstrap cycle.

use super::bootstrap::StartupSource;
use crate::engine::{EngineEntryMode, RuntimeLoadRequest};

/// The intent for how the engine should enter the game after bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupIntent {
    /// Enter normally according to the startup asset's configuration.
    Raw,
    /// Load from a runtime save and enter gameplay directly.
    LoadLatest,
}

/// Describes a startup cycle (source + intent).
#[derive(Debug, Clone)]
pub struct StartupRequest {
    pub source: StartupSource,
    pub intent: StartupIntent,
}

impl StartupRequest {
    /// Creates a new request for a shipped game startup.
    pub fn game() -> Self {
        Self {
            source: StartupSource::Game,
            intent: StartupIntent::Raw,
        }
    }

    /// Creates a new request for a playtest startup.
    pub fn playtest(payload_path: String) -> Self {
        Self {
            source: StartupSource::Playtest { payload_path },
            intent: StartupIntent::Raw,
        }
    }

    /// Converts this request for a runtime load, preserving the source.
    pub fn for_runtime_load(&self, _load_request: RuntimeLoadRequest) -> Self {
        Self {
            source: self.source.clone(),
            intent: StartupIntent::LoadLatest,
        }
    }
}

/// Determines the effective [`EngineEntryMode`] given a startup intent.
///
/// `LoadLatest` always returns [`EngineEntryMode::Playing`].
pub fn entry_mode_for_intent(
    intent: &StartupIntent,
    raw_entry_mode: EngineEntryMode,
) -> EngineEntryMode {
    match intent {
        StartupIntent::Raw => raw_entry_mode,
        StartupIntent::LoadLatest => EngineEntryMode::Playing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_load_request_preserves_game_source() {
        let request = StartupRequest::game();
        let load_request = RuntimeLoadRequest::Latest;

        let next = request.for_runtime_load(load_request);

        assert_eq!(next.source, StartupSource::Game);
        assert_eq!(next.intent, StartupIntent::LoadLatest);
    }

    #[test]
    fn runtime_load_request_preserves_playtest_source() {
        let request = StartupRequest::playtest("/tmp/test_payload.ron".to_string());
        let load_request = RuntimeLoadRequest::Latest;

        let next = request.for_runtime_load(load_request);

        match next.source {
            StartupSource::Playtest { payload_path } => {
                assert_eq!(payload_path, "/tmp/test_payload.ron");
            }
            _ => panic!("expected playtest source, got game source"),
        }
        assert_eq!(next.intent, StartupIntent::LoadLatest);
    }

    #[test]
    fn raw_startup_keeps_start_menu_entry_mode() {
        let raw_entry = EngineEntryMode::StartMenu {
            menu_id: "main_menu".to_string(),
        };

        let result = entry_mode_for_intent(&StartupIntent::Raw, raw_entry.clone());

        assert_eq!(result, raw_entry);
    }

    #[test]
    fn explicit_load_bypasses_start_menu_entry_mode() {
        let raw_entry = EngineEntryMode::StartMenu {
            menu_id: "main_menu".to_string(),
        };

        let result = entry_mode_for_intent(&StartupIntent::LoadLatest, raw_entry);

        assert_eq!(result, EngineEntryMode::Playing);
    }

    #[test]
    fn load_latest_intent_forces_gameplay_entry_mode() {
        let raw_entry = EngineEntryMode::StartMenu {
            menu_id: "main_menu".to_string(),
        };

        let result = entry_mode_for_intent(&StartupIntent::LoadLatest, raw_entry);

        assert_eq!(result, EngineEntryMode::Playing);
    }
}
