use super::{expand_chaos_profile, resolve_control_profile};
use engine_core::playtest::{
    ControlLoopPolicy, PlaytestControlProfile, PlaytestControlProfileRef, PlaytestControlRequest,
};
use engine_core::prelude::*;

/// Accepted playtest control request with a resolved replayable profile.
#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedPlaytestControlRequest {
    /// Normalized request preserved for manifests and future runtime reapplication.
    pub request: PlaytestControlRequest,
    /// Stable human-readable label for diagnostics and snapshots.
    pub profile_label: String,
    /// Concrete resolved profile ready for playback without further lookup.
    pub profile: PlaytestControlProfile,
}

/// Validates and resolves a playtest control request into a replayable profile.
pub fn accept_playtest_control_request(
    mut request: PlaytestControlRequest,
) -> Option<AcceptedPlaytestControlRequest> {
    let profile_label = control_profile_label(&request);

    if request.loop_policy != ControlLoopPolicy::RunOnce {
        onscreen_error!(
            "Rejected playtest control request for unsupported loop policy on {}",
            profile_label
        );
        return None;
    }

    let profile = if let Some(chaos) = request.chaos.as_ref() {
        let expanded = expand_chaos_profile(chaos);
        request.profile = PlaytestControlProfileRef::Inline(expanded.clone());
        expanded
    } else {
        let Some(profile) = resolve_control_profile(&request.profile) else {
            onscreen_error!(
                "Rejected playtest control request for unresolved profile {}",
                profile_label
            );
            return None;
        };
        profile
    };

    if profile.movement_frames.is_empty() && profile.camera_frames.is_empty() {
        onscreen_error!(
            "Rejected playtest control request for empty profile {}",
            profile_label
        );
        return None;
    }

    Some(AcceptedPlaytestControlRequest {
        request,
        profile_label,
        profile,
    })
}

fn control_profile_label(request: &PlaytestControlRequest) -> String {
    match &request.profile {
        PlaytestControlProfileRef::Named(name) => name.clone(),
        PlaytestControlProfileRef::Inline(_) => "inline".to_string(),
    }
}
