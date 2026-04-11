use game_lib::startup::{StartupController, StartupSource};

/// Minimal headless playtest session scaffolding.
pub struct HeadlessPlaytestSession {
    session_id: String,
}

impl HeadlessPlaytestSession {
    /// Creates a new headless session.
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    /// Returns the session directory name used for headless runs.
    pub fn session_dir_name(&self) -> String {
        format!("{}_agent", self.session_id)
    }

    /// Builds the startup controller for a headless playtest session.
    pub fn startup_controller(&self) -> StartupController {
        StartupController::new(StartupSource::Game)
    }
}

#[cfg(test)]
mod tests {
    use super::HeadlessPlaytestSession;

    #[test]
    fn headless_session_builds_session_dir_name() {
        let session = HeadlessPlaytestSession::new("session-1".to_string());

        assert_eq!(session.session_dir_name(), "session-1_agent");
    }
}
