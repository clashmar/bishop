use engine_core::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingPrefabTransition {
    Exit,
    OpenExisting(PrefabId),
    CreateBlank { name: String, initial_path: PathBuf },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrefabTransitionPrompt {
    None,
    Dirty,
    Empty,
}

#[derive(Default)]
pub(crate) struct PrefabSessionState {
    pending_transition: Option<PendingPrefabTransition>,
    require_picker: bool,
}

impl PrefabSessionState {
    #[cfg(test)]
    pub(crate) fn pending_transition(&self) -> Option<&PendingPrefabTransition> {
        self.pending_transition.as_ref()
    }

    pub(crate) fn set_pending_transition(&mut self, transition: PendingPrefabTransition) {
        self.pending_transition = Some(transition);
    }

    pub(crate) fn take_pending_transition(&mut self) -> Option<PendingPrefabTransition> {
        self.pending_transition.take()
    }

    pub(crate) fn clear_pending_transition(&mut self) {
        self.pending_transition = None;
    }

    pub(crate) fn require_picker(&self) -> bool {
        self.require_picker
    }

    pub(crate) fn set_require_picker(&mut self, require_picker: bool) {
        self.require_picker = require_picker;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_transition_round_trips_through_state_api() {
        let mut state = PrefabSessionState::default();
        let transition = PendingPrefabTransition::CreateBlank {
            name: "Fresh".to_string(),
            initial_path: PathBuf::from("test.prefab"),
        };

        state.set_pending_transition(transition.clone());

        assert_eq!(state.pending_transition(), Some(&transition));
        assert_eq!(state.take_pending_transition(), Some(transition));
        assert_eq!(state.pending_transition(), None);
    }

    #[test]
    fn require_picker_flag_updates() {
        let mut state = PrefabSessionState::default();

        assert!(!state.require_picker());

        state.set_require_picker(true);
        assert!(state.require_picker());

        state.set_require_picker(false);
        assert!(!state.require_picker());
    }
}
