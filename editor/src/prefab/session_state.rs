use engine_core::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PendingPrefabRequest {
    CaptureSelection(Entity),
    CreateBlank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingPrefabTransition {
    Exit,
    OpenExisting(PrefabId),
    CreateBlank(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrefabTransitionPrompt {
    None,
    Dirty,
    Empty,
}

#[derive(Default)]
pub(crate) struct PrefabSessionState {
    pending_request: Option<PendingPrefabRequest>,
    pending_transition: Option<PendingPrefabTransition>,
    require_picker: bool,
}

impl PrefabSessionState {
    #[cfg(test)]
    pub(crate) fn pending_request(&self) -> Option<PendingPrefabRequest> {
        self.pending_request
    }

    pub(crate) fn set_pending_request(&mut self, request: PendingPrefabRequest) {
        self.pending_request = Some(request);
    }

    pub(crate) fn take_pending_request(&mut self) -> Option<PendingPrefabRequest> {
        self.pending_request.take()
    }

    pub(crate) fn clear_pending_request(&mut self) {
        self.pending_request = None;
    }

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
    fn pending_request_round_trips_through_state_api() {
        let mut state = PrefabSessionState::default();
        let entity = Entity::null();

        state.set_pending_request(PendingPrefabRequest::CaptureSelection(entity));

        assert_eq!(
            state.pending_request(),
            Some(PendingPrefabRequest::CaptureSelection(entity))
        );
        assert_eq!(
            state.take_pending_request(),
            Some(PendingPrefabRequest::CaptureSelection(entity))
        );
        assert_eq!(state.pending_request(), None);
    }

    #[test]
    fn pending_transition_round_trips_through_state_api() {
        let mut state = PrefabSessionState::default();
        let transition = PendingPrefabTransition::CreateBlank("Fresh".to_string());

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
