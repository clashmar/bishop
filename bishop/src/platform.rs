/// Tracks a close request independently of explicit exit confirmation.
#[cfg(any(test, all(feature = "web-gl", target_os = "emscripten")))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloseState {
    close_requested: bool,
    exit_confirmed: bool,
}

#[cfg(any(test, all(feature = "web-gl", target_os = "emscripten")))]
impl CloseState {
    pub(crate) fn set_close_requested(&mut self, requested: bool) {
        self.close_requested = requested;
    }

    pub(crate) fn set_exit_confirmed(&mut self, confirmed: bool) {
        self.exit_confirmed = confirmed;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.close_requested && self.exit_confirmed
    }
}

/// Cross-backend application lifecycle controls.
pub trait PlatformControl {
    /// Ensures prior draw calls are submitted before state-dependent UI work.
    fn flush_pending_draws(&mut self);
    /// Records whether a close was requested.
    fn set_close_requested(&mut self, requested: bool);
    /// Records whether the application confirmed exit.
    fn set_exit_confirmed(&mut self, confirmed: bool);
}

#[cfg(test)]
mod tests {
    use super::CloseState;

    #[test]
    fn close_request_requires_explicit_exit_confirmation() {
        let mut close = CloseState::default();

        close.set_close_requested(true);

        assert!(!close.should_exit());
        close.set_exit_confirmed(true);
        assert!(close.should_exit());
    }

    #[test]
    fn exit_confirmation_without_close_request_does_not_stop_the_runner() {
        let mut close = CloseState::default();

        close.set_exit_confirmed(true);

        assert!(!close.should_exit());
    }
}
